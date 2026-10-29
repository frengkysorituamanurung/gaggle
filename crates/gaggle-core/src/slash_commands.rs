//! Slash command handling for the agent loop.
//!
//! Slash commands are special user inputs that start with `/` and are
//! processed by the agent before being sent to the model. They can trigger
//! actions like compaction, model switching, etc.

use serde::{Deserialize, Serialize};

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
    pool: &sqlx::SqlitePool,
) -> SlashCommandResult {
    match cmd.name.as_str() {
        "compact" => SlashCommandResult::Handled {
            output: "Compacting conversation history...".to_string(),
        },
        "clear" => SlashCommandResult::Handled {
            output: "Conversation cleared.".to_string(),
        },
        "model" => SlashCommandResult::Handled {
            output: format!("Model switching to: {}", cmd.args),
        },
        "provider" => SlashCommandResult::Handled {
            output: format!("Provider switching to: {}", cmd.args),
        },
        "help" => SlashCommandResult::Handled {
            output: "Available commands: /compact, /clear, /model, /provider, /help, /status, /tools, /memory, /recipe, /schedule".to_string(),
        },
        "status" => SlashCommandResult::Handled {
            output: format!("Chat: {} | Status: active", chat_id),
        },
        "tools" => SlashCommandResult::Handled {
            output: "Listing available tools...".to_string(),
        },
        "memory" => SlashCommandResult::Handled {
            output: "Memory management...".to_string(),
        },
        "recipe" => SlashCommandResult::Handled {
            output: "Recipe management...".to_string(),
        },
        "schedule" => SlashCommandResult::Handled {
            output: "Schedule management...".to_string(),
        },
        _ => SlashCommandResult::NotRecognized,
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
}
