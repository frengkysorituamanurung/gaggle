//! Slash command handling for the agent loop.
//!
//! Slash commands are special user inputs that start with `/` and are
//! processed by the agent before being sent to the model. They can trigger
//! actions like compaction, model switching, etc.

use crate::compaction::CompactionEngine;
use crate::moim::MoimManager;
use crate::provider::ModelProviderAdapter;
use gaggle_db::repositories::{
    chats::ChatRepository, extensions::ExtensionRepository, projects::ProjectRepository,
    providers::ProviderRepository, recipes::RecipeRepository,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A parsed slash command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    /// The command name (without the leading `/`).
    pub name: String,
    /// The arguments passed to the command.
    pub args: String,
    /// Whether this command was recognized.
    pub recognized: bool,
}

/// Result of executing a slash command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlashCommandResult {
    /// The command was handled and produced output.
    Handled { output: String },
    /// The command was not recognized; treat as normal input.
    NotRecognized,
    /// The command requests a context modification.
    ContextModification { modified_prompt: String },
}

/// Parse a user input for slash commands.
pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = trimmed[1..].splitn(2, ' ').collect();
    let name = parts[0].to_lowercase();
    let args = parts.get(1).unwrap_or(&"").to_string();

    let recognized = matches!(
        name.as_str(),
        "compact"
            | "clear"
            | "model"
            | "provider"
            | "help"
            | "status"
            | "tools"
            | "memory"
            | "recipe"
            | "schedule"
            | "instructions"
    );

    Some(SlashCommand {
        name,
        args,
        recognized,
    })
}

/// Execute a recognized slash command.
pub async fn execute_slash_command(
    cmd: &SlashCommand,
    chat_id: &str,
    pool: &SqlitePool,
    provider: Option<&dyn ModelProviderAdapter>,
) -> SlashCommandResult {
    let chat_repo = ChatRepository::new(pool.clone());
    let chat = match chat_repo.get_by_id(chat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return SlashCommandResult::Handled {
                output: format!("Chat not found: {}", chat_id),
            };
        }
        Err(e) => {
            return SlashCommandResult::Handled {
                output: format!("Failed to load chat: {}", e),
            };
        }
    };

    match cmd.name.as_str() {
        "help" => SlashCommandResult::Handled {
            output: "Available commands: /compact, /clear, /model [name], /provider, /help, /status, /tools, /memory [list|add|remove], /recipe, /schedule, /instructions".to_string(),
        },
        "status" => {
            let model_name = chat.model_id.clone().unwrap_or_else(|| "(default)".to_string());
            let msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            SlashCommandResult::Handled {
                output: format!(
                    "Chat: {} | Mode: {} | Model: {} | Messages: {}",
                    chat_id, chat.mode, model_name, msg_count
                ),
            }
        }
        "clear" => {
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    return SlashCommandResult::Handled {
                        output: format!("Failed to clear: {}", e),
                    }
                }
            };
            let _ = sqlx::query(
                "DELETE FROM message_parts WHERE message_id IN (SELECT id FROM messages WHERE chat_id = ?)",
            )
            .bind(chat_id)
            .execute(&mut *tx)
            .await;
            let _ = sqlx::query("DELETE FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .execute(&mut *tx)
                .await;
            let _ = sqlx::query("DELETE FROM chat_summaries WHERE chat_id = ?")
                .bind(chat_id)
                .execute(&mut *tx)
                .await;
            if let Err(e) = tx.commit().await {
                return SlashCommandResult::Handled {
                    output: format!("Failed to clear: {}", e),
                };
            }
            SlashCommandResult::Handled {
                output: "Conversation cleared.".to_string(),
            }
        }
        "model" => {
            let args = cmd.args.trim();
            if args.is_empty() {
                return SlashCommandResult::Handled {
                    output: format!("Current model: {}", chat.model_id.clone().unwrap_or_else(|| "(default)".to_string())),
                };
            }
            let provider_repo = ProviderRepository::new(pool.clone());
            match provider_repo.find_model_by_key(args).await {
                Ok(Some(model)) => {
                    match chat_repo.update_model(chat_id, Some(&model.0.id)).await {
                        Ok(_) => SlashCommandResult::Handled {
                            output: format!("Model switched to: {} ({})", model.0.display_name, model.0.id),
                        },
                        Err(e) => SlashCommandResult::Handled {
                            output: format!("Failed to update model: {}", e),
                        },
                    }
                }
                Ok(None) => SlashCommandResult::Handled {
                    output: format!("Model not found: {}", args),
                },
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to resolve model: {}", e),
                },
            }
        }
        "provider" => {
            let provider_repo = ProviderRepository::new(pool.clone());
            match provider_repo.list_providers().await {
                Ok(providers) => {
                    if providers.is_empty() {
                        return SlashCommandResult::Handled {
                            output: "No providers configured.".to_string(),
                        };
                    }
                    let mut lines = Vec::new();
                    for p in providers {
                        let models = provider_repo.list_models_by_provider(&p.id).await.unwrap_or_default();
                        lines.push(format!(
                            "{} ({}) - {} models",
                            p.name,
                            p.kind,
                            models.len()
                        ));
                    }
                    SlashCommandResult::Handled {
                        output: lines.join("\n"),
                    }
                }
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to list providers: {}", e),
                },
            }
        }
        "tools" => {
            let ext_repo = ExtensionRepository::new(pool.clone());
            let tools = ext_repo.list_enabled_tools().await.unwrap_or_default();
            if tools.is_empty() {
                return SlashCommandResult::Handled {
                    output: "No enabled tools.".to_string(),
                };
            }
            let names: Vec<String> = tools.iter().map(|t| t.tool_name.clone()).collect();
            SlashCommandResult::Handled {
                output: format!("Enabled tools ({}): {}", names.len(), names.join(", ")),
            }
        }
        "memory" | "instructions" => handle_memory_command(cmd, chat_id, pool).await,
        "recipe" => {
            let recipe_repo = RecipeRepository::new(pool.clone());
            match recipe_repo.list_all().await {
                Ok(recipes) => {
                    if recipes.is_empty() {
                        return SlashCommandResult::Handled {
                            output: "No recipes available.".to_string(),
                        };
                    }
                    let lines: Vec<String> = recipes
                        .iter()
                        .take(20)
                        .map(|r| format!("{} ({})", r.name, r.id))
                        .collect();
                    SlashCommandResult::Handled {
                        output: lines.join("\n"),
                    }
                }
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to list recipes: {}", e),
                },
            }
        }
        "schedule" => {
            let recipe_repo = RecipeRepository::new(pool.clone());
            let project_repo = ProjectRepository::new(pool.clone());
            let project_id = chat.project_id.clone();
            let _ = project_repo;
            match recipe_repo.list_schedules(Some(&project_id)).await {
                Ok(schedules) => {
                    if schedules.is_empty() {
                        return SlashCommandResult::Handled {
                            output: "No schedules configured.".to_string(),
                        };
                    }
                    let lines: Vec<String> = schedules
                        .iter()
                        .map(|s| format!("recipe {} schedule {} enabled={}", s.recipe_id, s.id, s.enabled))
                        .collect();
                    SlashCommandResult::Handled {
                        output: lines.join("\n"),
                    }
                }
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to list schedules: {}", e),
                },
            }
        }
        "compact" => {
            let Some(provider) = provider else {
                return SlashCommandResult::Handled {
                    output: "Compaction requires an active provider.".to_string(),
                };
            };
            let context_window: i64 = 128_000;
            match CompactionEngine::compact_chat(
                pool.clone(),
                chat_id,
                context_window,
                provider,
                "manual",
                true,
            )
            .await
            {
                Ok(Some(result)) => SlashCommandResult::Handled {
                    output: format!(
                        "Compacted conversation. Tokens: {} -> {}. Covers {} messages.",
                        result.tokens_before, result.tokens_after, result.covers_until_seq
                    ),
                },
                Ok(None) => SlashCommandResult::Handled {
                    output: "Nothing to compact.".to_string(),
                },
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Compaction failed: {}", e),
                },
            }
        }
        _ => SlashCommandResult::NotRecognized,
    }
}

async fn handle_memory_command(
    cmd: &SlashCommand,
    chat_id: &str,
    pool: &SqlitePool,
) -> SlashCommandResult {
    let moim = MoimManager::new(pool.clone());
    let parts: Vec<&str> = cmd.args.splitn(2, ' ').collect();
    let sub = parts.first().copied().unwrap_or("").to_lowercase();
    let rest = parts.get(1).copied().unwrap_or("").trim();

    match sub.as_str() {
        "" | "list" => match moim.list_instructions(chat_id).await {
            Ok(items) => {
                if items.is_empty() {
                    SlashCommandResult::Handled {
                        output: "No model instructions.".to_string(),
                    }
                } else {
                    let lines: Vec<String> = items
                        .iter()
                        .map(|i| format!("[{}] {} (priority {})", i.id, i.instruction, i.priority))
                        .collect();
                    SlashCommandResult::Handled {
                        output: lines.join("\n"),
                    }
                }
            }
            Err(e) => SlashCommandResult::Handled {
                output: format!("Failed to list instructions: {}", e),
            },
        },
        "add" => {
            if rest.is_empty() {
                return SlashCommandResult::Handled {
                    output: "Usage: /memory add <instruction>".to_string(),
                };
            }
            match moim.add_instruction(chat_id, rest, 0).await {
                Ok(instr) => SlashCommandResult::Handled {
                    output: format!("Added instruction: {} ({})", instr.instruction, instr.id),
                },
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to add instruction: {}", e),
                },
            }
        }
        "remove" | "delete" => {
            if rest.is_empty() {
                return SlashCommandResult::Handled {
                    output: "Usage: /memory remove <id>".to_string(),
                };
            }
            match moim.remove_instruction(rest).await {
                Ok(_) => SlashCommandResult::Handled {
                    output: format!("Removed instruction: {}", rest),
                },
                Err(e) => SlashCommandResult::Handled {
                    output: format!("Failed to remove instruction: {}", e),
                },
            }
        }
        _ => SlashCommandResult::Handled {
            output: "Usage: /memory [list|add <text>|remove <id>]".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_recognized_command() {
        let cmd = parse_slash_command("/compact").unwrap();
        assert_eq!(cmd.name, "compact");
        assert_eq!(cmd.args, "");
        assert!(cmd.recognized);
    }

    #[test]
    fn parse_command_with_args() {
        let cmd = parse_slash_command("/model gpt-4").unwrap();
        assert_eq!(cmd.name, "model");
        assert_eq!(cmd.args, "gpt-4");
        assert!(cmd.recognized);
    }

    #[test]
    fn parse_unrecognized_command() {
        let cmd = parse_slash_command("/unknown").unwrap();
        assert_eq!(cmd.name, "unknown");
        assert!(!cmd.recognized);
    }

    #[test]
    fn parse_non_slash_input() {
        assert!(parse_slash_command("hello world").is_none());
    }

    #[test]
    fn instructions_recognized() {
        let cmd = parse_slash_command("/instructions list").unwrap();
        assert!(cmd.recognized);
    }
}
