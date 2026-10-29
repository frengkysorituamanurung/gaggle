//! Operation pipeline foundation (Phase 1).
//!
//! Provides discrete runtime operations while `AgentRuntime::run_turn` remains
//! the public facade and durable event persistence stays the source of truth.

use crate::{
    CancellationToken,
    provider::{
        ModelCompletionRequest, ModelCompletionResponse, ModelProviderAdapter, ModelStreamEvent,
        ProviderError, TokenUsage,
    },
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use std::future::Future;

#[derive(Debug)]
pub enum InferenceOperationError {
    Provider(ProviderError),
    Persistence(anyhow::Error),
    Cancelled,
}

pub struct InferenceOutput {
    pub completion: ModelCompletionResponse,
    pub used_native_stream: bool,
}

pub struct InferenceOperation;

impl InferenceOperation {
    pub async fn run<F, Fut>(
        provider: &dyn ModelProviderAdapter,
        request: ModelCompletionRequest,
        cancel_token: &CancellationToken,
        mut on_text_delta: F,
    ) -> std::result::Result<InferenceOutput, InferenceOperationError>
    where
        F: FnMut(String) -> Fut + Send,
        Fut: Future<Output = Result<()>> + Send,
    {
        let Some(mut stream) = provider.stream(request.clone()) else {
            return tokio::select! {
                biased;
                _ = cancel_token.cancelled() => Err(InferenceOperationError::Cancelled),
                result = provider.complete(&request) => result
                    .map(|completion| InferenceOutput {
                        completion,
                        used_native_stream: false,
                    })
                    .map_err(InferenceOperationError::Provider),
            };
        };

        let mut streamed_text = String::new();
        let mut streamed_calls = Vec::new();
        let mut streamed_usage = TokenUsage::default();
        let mut done_response = None;

        loop {
            let item = tokio::select! {
                biased;
                _ = cancel_token.cancelled() => return Err(InferenceOperationError::Cancelled),
                item = stream.next() => item,
            };
            let Some(item) = item else { break };
            match item {
                Ok(ModelStreamEvent::TextDelta(delta)) => {
                    if !delta.is_empty() {
                        on_text_delta(delta.clone())
                            .await
                            .map_err(InferenceOperationError::Persistence)?;
                        streamed_text.push_str(&delta);
                    }
                }
                Ok(ModelStreamEvent::ToolCall(call)) => streamed_calls.push(call),
                Ok(ModelStreamEvent::Usage(usage)) => streamed_usage = usage,
                Ok(ModelStreamEvent::Done(response)) => done_response = Some(response),
                Err(error) => return Err(InferenceOperationError::Provider(error)),
            }
        }

        let completion = match done_response {
            Some(response) if response.text.is_empty() && !streamed_text.is_empty() => {
                ModelCompletionResponse {
                    text: streamed_text,
                    ..response
                }
            }
            Some(response) if response.tool_calls.is_empty() && !streamed_calls.is_empty() => {
                ModelCompletionResponse {
                    tool_calls: streamed_calls,
                    ..response
                }
            }
            Some(response)
                if response.usage == TokenUsage::default()
                    && streamed_usage != TokenUsage::default() =>
            {
                ModelCompletionResponse {
                    usage: streamed_usage,
                    ..response
                }
            }
            Some(response) => response,
            None => ModelCompletionResponse {
                text: streamed_text,
                tool_calls: streamed_calls,
                usage: streamed_usage,
            },
        };

        Ok(InferenceOutput {
            completion,
            used_native_stream: true,
        })
    }
}

/// Outcome of a single operation.
#[derive(Debug, Clone)]
pub enum OperationOutcome {
    Continue,
    WaitingApproval,
    Completed { final_text: String },
    Cancelled,
    Failed { error: String },
}

/// Shared context passed to every operation.
#[derive(Clone)]
pub struct OperationContext {
    pub run_id: String,
    pub chat_id: String,
    pub project_id: String,
    pub step_index: i64,
    pub extra: Value,
}

#[async_trait]
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&mut self, ctx: &OperationContext) -> Result<OperationOutcome>;
}

/// Sequential runner for a list of operations. Stops early on terminal outcomes.
pub struct OperationRunner {
    operations: Vec<Box<dyn Operation>>,
}

impl OperationRunner {
    pub fn new(operations: Vec<Box<dyn Operation>>) -> Self {
        Self { operations }
    }

    pub async fn run(&mut self, ctx: &OperationContext) -> Result<OperationOutcome> {
        for op in self.operations.iter_mut() {
            match op.run(ctx).await? {
                OperationOutcome::Continue => continue,
                terminal => return Ok(terminal),
            }
        }
        Ok(OperationOutcome::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ModelCompletionStream, ProposedToolCall};
    use futures::stream;
    use std::sync::{Arc, Mutex};

    struct StreamingProvider {
        events: Vec<std::result::Result<ModelStreamEvent, ProviderError>>,
        pending: bool,
    }

    #[async_trait::async_trait]
    impl ModelProviderAdapter for StreamingProvider {
        fn provider_name(&self) -> &str {
            "streaming-test"
        }

        async fn complete(
            &self,
            _request: &ModelCompletionRequest,
        ) -> std::result::Result<ModelCompletionResponse, ProviderError> {
            Err(ProviderError::Transport {
                provider: "streaming-test".to_string(),
                message: "streaming test provider does not support complete".to_string(),
            })
        }

        fn stream(&self, _request: ModelCompletionRequest) -> Option<ModelCompletionStream> {
            let stream: ModelCompletionStream = if self.pending {
                Box::pin(stream::pending())
            } else {
                Box::pin(stream::iter(self.events.clone()))
            };
            Some(stream)
        }
    }

    struct NoopOp;
    #[async_trait]
    impl Operation for NoopOp {
        fn name(&self) -> &'static str {
            "noop"
        }
        async fn run(&mut self, _ctx: &OperationContext) -> Result<OperationOutcome> {
            Ok(OperationOutcome::Continue)
        }
    }

    #[tokio::test]
    async fn runner_completes_noops() {
        let mut runner = OperationRunner::new(vec![Box::new(NoopOp), Box::new(NoopOp)]);
        let ctx = OperationContext {
            run_id: "r".to_string(),
            chat_id: "c".to_string(),
            project_id: "p".to_string(),
            step_index: 1,
            extra: Value::Null,
        };
        let out = runner.run(&ctx).await.unwrap();
        assert!(matches!(out, OperationOutcome::Continue));
    }

    #[tokio::test]
    async fn inference_operation_emits_deltas_and_merges_streamed_response() {
        let provider = StreamingProvider {
            pending: false,
            events: vec![
                Ok(ModelStreamEvent::TextDelta("hello".to_string())),
                Ok(ModelStreamEvent::ToolCall(ProposedToolCall {
                    id: "call-1".to_string(),
                    tool_name: "test.tool".to_string(),
                    arguments_json: "{}".to_string(),
                })),
                Ok(ModelStreamEvent::Done(ModelCompletionResponse {
                    text: String::new(),
                    tool_calls: Vec::new(),
                    usage: TokenUsage::default(),
                })),
            ],
        };
        let deltas = Arc::new(Mutex::new(Vec::new()));
        let request = ModelCompletionRequest {
            model_key: "test-model".to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_output_tokens: None,
        };

        let output = InferenceOperation::run(&provider, request, &CancellationToken::new(), {
            let deltas = deltas.clone();
            move |delta| {
                let deltas = deltas.clone();
                async move {
                    deltas
                        .lock()
                        .map_err(|_| anyhow!("delta collector lock poisoned"))?
                        .push(delta);
                    Ok(())
                }
            }
        })
        .await
        .unwrap();

        assert!(output.used_native_stream);
        assert_eq!(output.completion.text, "hello");
        assert_eq!(output.completion.tool_calls.len(), 1);
        assert_eq!(*deltas.lock().unwrap(), vec!["hello"]);
    }

    #[tokio::test]
    async fn inference_operation_cancels_a_pending_stream() {
        let provider = StreamingProvider {
            events: Vec::new(),
            pending: true,
        };
        let cancellation = CancellationToken::new();
        let cancellation_task_token = cancellation.clone();
        let cancellation_task = tokio::spawn(async move {
            tokio::task::yield_now().await;
            cancellation_task_token.cancel();
        });
        let request = ModelCompletionRequest {
            model_key: "test-model".to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_output_tokens: None,
        };

        let result =
            InferenceOperation::run(&provider, request, &cancellation, |_| async { Ok(()) }).await;
        cancellation_task.await.unwrap();

        assert!(matches!(result, Err(InferenceOperationError::Cancelled)));
    }
}
