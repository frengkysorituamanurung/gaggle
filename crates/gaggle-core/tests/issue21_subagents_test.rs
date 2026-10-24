#![allow(clippy::field_reassign_with_default)]

use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    CancellationToken, ModelCompletionRequest, ModelCompletionResponse, ModelProviderAdapter,
    ProviderError, SubagentManager, SubagentSpawnRequest, TokenUsage, ToolExecutor,
};
use gaggle_db::{
    connect_in_memory,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

struct DummyExecutor;
#[async_trait]
impl ToolExecutor for DummyExecutor {
    async fn execute_tool(
        &self,
        _run_id: &str,
        _chat: &gaggle_db::models::Chat,
        _project_root: &str,
        _tool_name: &str,
        _args_json: &str,
    ) -> Result<String, String> {
        Ok("{}".to_string())
    }
}

struct SimpleProvider {
    text: String,
}
#[async_trait]
impl ModelProviderAdapter for SimpleProvider {
    fn provider_name(&self) -> &str {
        "simple"
    }
    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        Ok(ModelCompletionResponse {
            text: self.text.clone(),
            tool_calls: vec![],
            usage: TokenUsage::default(),
        })
    }
}

struct DelayProvider {
    delay: Duration,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
}
#[async_trait]
impl ModelProviderAdapter for DelayProvider {
    fn provider_name(&self) -> &str {
        "delay"
    }
    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let current = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(current, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(ModelCompletionResponse {
            text: "delayed done".to_string(),
            tool_calls: vec![],
            usage: TokenUsage::default(),
        })
    }
}

#[tokio::test]
async fn test_subagent_spawn_creates_child_chat_and_parent_summary() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "ask")
        .await?;

    let manager = SubagentManager::new(pool.clone(), 2);
    let provider = SimpleProvider {
        text: "child summary".to_string(),
    };
    let executor = DummyExecutor;
    let cancel = CancellationToken::new();

    let mut req = SubagentSpawnRequest::default();
    req.parent_chat_id = parent.id.clone();
    req.parent_tool_call_id = Some("tool_call_123".to_string());
    req.title = "child".to_string();
    req.prompt = "do work".to_string();

    let outcome = manager
        .spawn_and_run(&parent.id, &provider, &executor, &cancel, req)
        .await?;

    assert_eq!(outcome.child_chat.chat_type, "subagent");
    assert_eq!(
        outcome.child_chat.parent_chat_id.as_deref(),
        Some(parent.id.as_str())
    );
    assert_eq!(
        outcome.child_chat.parent_tool_call_id.as_deref(),
        Some("tool_call_123")
    );
    assert_eq!(outcome.status, "completed");
    assert_eq!(outcome.summary, "child summary");
    assert_eq!(outcome.final_text, "child summary");

    // Child has its own conversation history.
    let messages = chat_repo
        .list_messages_with_parts(&outcome.child_chat.id, 10)
        .await?;
    assert!(!messages.is_empty());

    // Parent unchanged type.
    let parent_after = chat_repo.get_by_id(&parent.id).await?.unwrap();
    assert_eq!(parent_after.chat_type, "user");

    let _ = RunRepository::new(pool.clone());
    Ok(())
}

#[tokio::test]
async fn test_subagent_cancellation_propagates() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p2", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "ask")
        .await?;

    let manager = SubagentManager::new(pool.clone(), 2);
    let provider = SimpleProvider {
        text: "should not finish".to_string(),
    };
    let executor = DummyExecutor;
    let cancel = CancellationToken::new();
    cancel.cancel();

    let mut req = SubagentSpawnRequest::default();
    req.title = "child".to_string();
    req.prompt = "do work".to_string();

    let outcome = manager
        .spawn_and_run(&parent.id, &provider, &executor, &cancel, req)
        .await?;

    assert_eq!(outcome.status, "cancelled");
    assert!(cancel.is_cancelled());
    Ok(())
}

#[tokio::test]
async fn test_subagent_bounded_concurrency() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p3", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "ask")
        .await?;

    let manager = Arc::new(SubagentManager::new(pool.clone(), 1));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(DelayProvider {
        delay: Duration::from_millis(100),
        active: active.clone(),
        max_active: max_active.clone(),
    });
    let executor = Arc::new(DummyExecutor);
    let cancel = CancellationToken::new();

    let start = Instant::now();
    let mut handles = Vec::new();
    for i in 0..2 {
        let manager = manager.clone();
        let provider = provider.clone();
        let executor = executor.clone();
        let cancel = cancel.clone();
        let parent_id = parent.id.clone();
        handles.push(tokio::spawn(async move {
            let mut req = SubagentSpawnRequest::default();
            req.title = format!("child{i}");
            req.prompt = "do work".to_string();
            manager
                .spawn_and_run(
                    &parent_id,
                    provider.as_ref(),
                    executor.as_ref(),
                    &cancel,
                    req,
                )
                .await
        }));
    }
    for h in handles {
        let res = h.await.unwrap()?;
        assert_eq!(res.status, "completed");
    }
    let elapsed = start.elapsed();
    assert!(
        max_active.load(Ordering::SeqCst) <= 1,
        "max active {} > 1",
        max_active.load(Ordering::SeqCst)
    );
    assert!(elapsed >= Duration::from_millis(180));
    Ok(())
}

#[tokio::test]
async fn test_subagent_sequential_when_repeated() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p4", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "ask")
        .await?;

    let manager = Arc::new(SubagentManager::new(pool.clone(), 4));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(DelayProvider {
        delay: Duration::from_millis(50),
        active: active.clone(),
        max_active: max_active.clone(),
    });
    let executor = Arc::new(DummyExecutor);
    let cancel = CancellationToken::new();

    let mut handles = Vec::new();
    for i in 0..2 {
        let manager = manager.clone();
        let provider = provider.clone();
        let executor = executor.clone();
        let cancel = cancel.clone();
        let parent_id = parent.id.clone();
        handles.push(tokio::spawn(async move {
            let mut req = SubagentSpawnRequest::default();
            req.title = format!("child{i}");
            req.prompt = "do work".to_string();
            req.sequential_when_repeated = true;
            manager
                .spawn_and_run(
                    &parent_id,
                    provider.as_ref(),
                    executor.as_ref(),
                    &cancel,
                    req,
                )
                .await
        }));
    }
    for h in handles {
        let res = h.await.unwrap()?;
        assert_eq!(res.status, "completed");
    }
    assert!(
        max_active.load(Ordering::SeqCst) <= 1,
        "sequential max active {} > 1",
        max_active.load(Ordering::SeqCst)
    );
    Ok(())
}

#[tokio::test]
async fn test_subagent_depth_limit() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p5", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "ask")
        .await?;

    // Create a child manually to make parent depth 1.
    let child = chat_repo
        .create_subagent_chat(&proj.id, "child", "local", "ask", &parent.id, None)
        .await?;

    let manager = SubagentManager::new(pool.clone(), 2);
    // max_depth default 1: child cannot spawn.
    let provider = SimpleProvider {
        text: "x".to_string(),
    };
    let executor = DummyExecutor;
    let cancel = CancellationToken::new();
    let mut req = SubagentSpawnRequest::default();
    req.title = "grandchild".to_string();
    req.prompt = "do work".to_string();
    let res = manager
        .spawn_and_run(&child.id, &provider, &executor, &cancel, req)
        .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("depth limit"));
    Ok(())
}

#[tokio::test]
async fn test_subagent_inherits_permission_mode() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let proj = proj_repo.create("p6", "/tmp", "none").await?;
    let parent = chat_repo
        .create(&proj.id, "parent", "user", "local", "full")
        .await?;

    let manager = SubagentManager::new(pool.clone(), 2);
    let provider = SimpleProvider {
        text: "ok".to_string(),
    };
    let executor = DummyExecutor;
    let cancel = CancellationToken::new();
    let mut req = SubagentSpawnRequest::default();
    req.title = "child".to_string();
    req.prompt = "do work".to_string();
    let outcome = manager
        .spawn_and_run(&parent.id, &provider, &executor, &cancel, req)
        .await?;
    assert_eq!(outcome.child_chat.permission_mode, "full");
    Ok(())
}
