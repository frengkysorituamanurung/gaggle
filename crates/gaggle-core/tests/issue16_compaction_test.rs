use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    CompactionEngine, ModelCompletionResponse, ModelProviderAdapter, ProviderError, TokenUsage,
    context_assembler::estimate_tokens, should_compact,
};
use gaggle_db::{
    connect_in_memory,
    repositories::{chats::ChatRepository, projects::ProjectRepository},
};

struct SummaryProvider;
#[async_trait]
impl ModelProviderAdapter for SummaryProvider {
    fn provider_name(&self) -> &str {
        "summary-mock"
    }
    async fn complete(
        &self,
        _req: &gaggle_core::ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        Ok(ModelCompletionResponse {
            text: "Compacted summary of history.".to_string(),
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 5,
                output_tokens: 3,
                cached_tokens: 0,
            },
        })
    }
}

#[test]
fn test_should_compact_threshold() {
    assert!(!should_compact(10, 100, 0.75));
    assert!(should_compact(75, 100, 0.75));
    assert!(should_compact(80, 100, 0.75));
}

#[tokio::test]
async fn test_compaction_storage_and_context() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let project = proj_repo.create("p", "/tmp/x", "none").await?;
    let chat = chat_repo
        .create(&project.id, "c", "user", "local", "ask")
        .await?;
    for i in 0..5 {
        chat_repo
            .append_message(&chat.id, None, "user", &format!("message {}", i))
            .await?;
    }
    // force threshold low by using small context window
    let provider = SummaryProvider;
    let result =
        CompactionEngine::maybe_compact_chat(pool.clone(), &chat.id, 10, &provider, "manual")
            .await?;
    assert!(result.is_some());
    let cr = result.unwrap();
    assert!(cr.compacted);
    assert_eq!(cr.covers_until_seq, 5);
    let summaries = chat_repo.list_summaries(&chat.id).await?;
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].trigger, "manual");
    assert!(summaries[0].tokens_before >= summaries[0].tokens_after);
    // context replacement
    let assembler = gaggle_core::ContextAssembler::new(pool.clone());
    // set model context window large
    let _ = estimate_tokens("x");
    let ctx = assembler.assemble(&chat.id, "new prompt").await?;
    let all = ctx
        .messages
        .iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(all.contains("Compacted summary"));
    Ok(())
}
