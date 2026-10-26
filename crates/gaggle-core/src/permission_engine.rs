use anyhow::{Result, bail};
use gaggle_db::{
    CorrelationIds,
    models::{Approval, CommandRule, ExtensionTool, ToolCall},
    repositories::permissions::PermissionRepository,
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
    NeedsApproval { kind: String, summary: String },
}

impl PermissionDecision {
    /// Stable label for structured logs and audit output.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny { .. } => "deny",
            Self::NeedsApproval { .. } => "needs_approval",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub project_id: Option<String>,
    pub permission_mode: String, // "full" | "ask" | "smart" | "chat_only"
    pub extension_tool: ExtensionTool,
    pub args_json: String,
    pub shell_command: Option<String>,
}

/// Evaluates whether a tool call can proceed, following the 7-step decision order from the Gaggle ERD:
/// 1. `chat_only`: reject all tools.
/// 2. `command_rules` (for shell): `deny` action always rejects, even in `full` mode.
/// 3. `full`: execute immediately.
/// 4. `tool_permissions`: `never_allow` rejects, `always_allow` executes, `ask_before` requests approval.
/// 5. Read-only annotation (`extension_tools.is_read_only == 1`): execute without approval.
/// 6. `smart` mode only: check `tool_risk_cache`; if read-only, allow.
/// 7. Otherwise, require approval (`pending` row in `approvals`).
pub struct PermissionEngine {
    repo: PermissionRepository,
    correlation: CorrelationIds,
}

impl PermissionEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            repo: PermissionRepository::new(pool),
            correlation: CorrelationIds::default(),
        }
    }

    /// Attaches chat/run/tool correlation identifiers for structured permission logs.
    pub fn with_correlation(mut self, correlation: CorrelationIds) -> Self {
        self.correlation = correlation;
        self
    }

    pub fn fingerprint_args(args_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(args_json.as_bytes());
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest.as_slice() {
            use std::fmt::Write as _;
            let _ = write!(&mut hex, "{byte:02x}");
        }
        hex
    }

    fn matches_command_rule(rule: &CommandRule, cmd: &str) -> bool {
        match rule.match_kind.as_str() {
            "exact" => cmd == rule.pattern,
            "prefix" => cmd.starts_with(&rule.pattern),
            "regex" => cmd.contains(&rule.pattern),
            _ => false,
        }
    }

    pub async fn evaluate(&self, ctx: &ToolCallContext) -> Result<PermissionDecision> {
        let fingerprint = Self::fingerprint_args(&ctx.args_json);
        tracing::debug!(
            target: "gaggle.permissions",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %self.correlation.run_id_field(),
            tool_call_id = %self.correlation.tool_call_id_field(),
            tool_name = %ctx.extension_tool.tool_name,
            permission_mode = %ctx.permission_mode,
            args_fingerprint = %fingerprint,
            has_shell_command = ctx.shell_command.is_some(),
            "Evaluating tool permission"
        );

        let decision = self.evaluate_inner(ctx).await?;

        tracing::info!(
            target: "gaggle.permissions",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %self.correlation.run_id_field(),
            tool_call_id = %self.correlation.tool_call_id_field(),
            tool_name = %ctx.extension_tool.tool_name,
            permission_mode = %ctx.permission_mode,
            decision = %decision.kind(),
            args_fingerprint = %fingerprint,
            "Tool permission decided"
        );

        Ok(decision)
    }

    async fn evaluate_inner(&self, ctx: &ToolCallContext) -> Result<PermissionDecision> {
        // 1. chat_only: all tools rejected
        if ctx.permission_mode == "chat_only" {
            return Ok(PermissionDecision::Deny {
                reason: "Chat is in chat_only permission mode".to_string(),
            });
        }

        // 2. command_rules (shell): deny always applies, even in `full` mode
        let mut matched_shell_rule: Option<CommandRule> = None;
        if let Some(cmd) = &ctx.shell_command {
            let rules = self
                .repo
                .list_command_rules(ctx.project_id.as_deref())
                .await?;
            for rule in rules {
                if Self::matches_command_rule(&rule, cmd) {
                    if rule.action == "deny" {
                        return Ok(PermissionDecision::Deny {
                            reason: format!("Blocked by command rule pattern '{}'", rule.pattern),
                        });
                    }
                    if matched_shell_rule.is_none() {
                        matched_shell_rule = Some(rule);
                    }
                }
            }
        }

        // 3. full: execute immediately
        if ctx.permission_mode == "full" {
            return Ok(PermissionDecision::Allow);
        }

        // Check if an explicit allow/ask command rule matched for shell
        if let Some(rule) = matched_shell_rule {
            match rule.action.as_str() {
                "allow" => return Ok(PermissionDecision::Allow),
                "ask" => {
                    return Ok(PermissionDecision::NeedsApproval {
                        kind: "shell".to_string(),
                        summary: format!("Shell command requires approval: {}", rule.pattern),
                    });
                }
                _ => {}
            }
        }

        // 4. tool_permissions: never_allow, always_allow, ask_before
        if let Some(perm) = self
            .repo
            .get_tool_permission(ctx.project_id.as_deref(), &ctx.extension_tool.id)
            .await?
        {
            match perm.level.as_str() {
                "never_allow" => {
                    return Ok(PermissionDecision::Deny {
                        reason: format!(
                            "Tool '{}' is set to never_allow",
                            ctx.extension_tool.tool_name
                        ),
                    });
                }
                "always_allow" => return Ok(PermissionDecision::Allow),
                "ask_before" => {
                    return Ok(PermissionDecision::NeedsApproval {
                        kind: Self::infer_approval_kind(&ctx.extension_tool.tool_name),
                        summary: format!("Approval required for {}", ctx.extension_tool.tool_name),
                    });
                }
                _ => {}
            }
        }

        // 5. Tool with read-only annotation runs without approval
        if ctx.extension_tool.is_read_only == 1 {
            return Ok(PermissionDecision::Allow);
        }

        // 6. Smart mode: check tool_risk_cache
        if ctx.permission_mode == "smart" {
            let fp = Self::fingerprint_args(&ctx.args_json);
            if let Some(cached) = self
                .repo
                .get_risk_cache(&ctx.extension_tool.tool_name, &fp)
                .await?
                && cached.is_read_only == 1
            {
                return Ok(PermissionDecision::Allow);
            }
        }

        // 7. Default: create pending approval and wait for UI decision
        Ok(PermissionDecision::NeedsApproval {
            kind: Self::infer_approval_kind(&ctx.extension_tool.tool_name),
            summary: format!("Execute tool '{}'", ctx.extension_tool.tool_name),
        })
    }

    /// Verifies that a resolved `Approval` is strictly bound to `tool_call` and that the
    /// proposed arguments and shell command have not been altered since approval was requested.
    pub fn verify_approved_execution(
        approval: &Approval,
        tool_call: &ToolCall,
        current_args_json: &str,
        current_shell_command: Option<&str>,
    ) -> Result<()> {
        if approval.status != "approved" {
            bail!(
                "Cannot execute tool call '{}': approval '{}' has status '{}'",
                tool_call.id,
                approval.id,
                approval.status
            );
        }

        if approval.tool_call_id != tool_call.id {
            bail!(
                "Approval '{}' is bound to tool_call '{}', not '{}'",
                approval.id,
                approval.tool_call_id,
                tool_call.id
            );
        }

        if Self::fingerprint_args(&tool_call.args) != Self::fingerprint_args(current_args_json) {
            bail!(
                "Approval replay rejected: tool arguments for '{}' changed after approval",
                tool_call.id
            );
        }

        if let Some(approved_cmd) = &approval.command
            && Some(approved_cmd.as_str()) != current_shell_command
        {
            bail!(
                "Approval replay rejected: shell command for '{}' does not match approved command",
                tool_call.id
            );
        }

        Ok(())
    }

    pub fn infer_approval_kind(tool_name: &str) -> String {
        if tool_name.starts_with("shell") {
            "shell".to_string()
        } else if tool_name.starts_with("browser") {
            "browser".to_string()
        } else if tool_name.starts_with("web") || tool_name.starts_with("net") {
            "net".to_string()
        } else {
            "fs_write".to_string()
        }
    }
}
