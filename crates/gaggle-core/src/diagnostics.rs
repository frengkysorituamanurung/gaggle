use crate::observability::{redact_log_text, truncate_for_message};
use anyhow::Result;
use gaggle_db::connection::now_ms;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

/// Identity of the running binary, independent of the platform that built it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildIdentity {
    pub service: String,
    pub version: String,
    /// Git commit injected by the packaging pipeline, or `unknown` for local builds.
    pub commit: String,
    /// Rust compiler version injected by the packaging pipeline, or `unknown`.
    pub rustc: String,
    /// `debug` or `release`.
    pub profile: String,
    /// `arch-os` target identifier (for example `aarch64-macos`).
    pub target: String,
}

impl BuildIdentity {
    pub fn current() -> Self {
        Self {
            service: "gaggle".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: option_env!("GAGGLE_GIT_COMMIT")
                .unwrap_or("unknown")
                .to_string(),
            rustc: option_env!("GAGGLE_RUSTC_VERSION")
                .unwrap_or("unknown")
                .to_string(),
            profile: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            },
            target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        }
    }
}

/// Result of one core health probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Failed,
}

impl HealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
        }
    }
}

/// A single named health check with a non-sensitive explanation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub detail: String,
}

/// Aggregated health of the local core (database, migrations, PRAGMAs, schema).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoreHealth {
    pub status: HealthStatus,
    pub checks: Vec<HealthCheck>,
}

impl CoreHealth {
    fn from_checks(checks: Vec<HealthCheck>) -> Self {
        let status = if checks
            .iter()
            .any(|check| check.status == HealthStatus::Failed)
        {
            HealthStatus::Failed
        } else if checks
            .iter()
            .any(|check| check.status == HealthStatus::Degraded)
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Ok
        };
        Self { status, checks }
    }
}

/// Availability of an optional capability and why it is unavailable or unverified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    NeedsSetup,
    Unavailable,
    /// Intentionally never probed, for example a network service that would cause egress.
    Unverified,
}

impl CapabilityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NeedsSetup => "needs_setup",
            Self::Unavailable => "unavailable",
            Self::Unverified => "unverified",
        }
    }
}

/// One optional capability reported by `gaggle doctor`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityStatus {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub state: CapabilityState,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

impl CapabilityStatus {
    fn new(
        id: &str,
        label: &str,
        state: CapabilityState,
        reason: impl Into<String>,
        action: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            available: state == CapabilityState::Available,
            state,
            reason: reason.into(),
            action: action.map(str::to_string),
        }
    }
}

/// Redacted, non-sensitive snapshot of the local Gaggle installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub service: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub db_path: String,
    pub db_healthy: bool,
    pub journal_mode: String,
    pub foreign_keys_enabled: bool,
    pub applied_migrations: i64,
    pub active_projects_count: i64,
    pub enabled_extensions_count: i64,
    pub pending_approvals_count: i64,
    /// Build identity (version, commit, profile, target).
    pub build: BuildIdentity,
    /// Core health checks with actionable failure details.
    pub core_health: CoreHealth,
    /// Optional capabilities, including why each one is unavailable.
    pub capabilities: Vec<CapabilityStatus>,
    /// Unix epoch milliseconds when the report was generated.
    pub generated_at: i64,
}

/// A single recent failed or paused run, with no prompts or tool output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureSummary {
    pub run_id: String,
    pub chat_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_category: Option<String>,
    pub message: String,
    pub resume_count: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

/// Explicit privacy statement embedded in an exported diagnostics bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacySummary {
    pub telemetry_enabled: bool,
    pub contains_prompts: bool,
    pub contains_source_files: bool,
    pub contains_raw_tool_output: bool,
    pub contains_secrets: bool,
    pub redaction_scope: String,
    pub generated_by_user_action: bool,
}

impl Default for PrivacySummary {
    fn default() -> Self {
        Self {
            telemetry_enabled: false,
            contains_prompts: false,
            contains_source_files: false,
            contains_raw_tool_output: false,
            contains_secrets: false,
            redaction_scope: "tracing-writer + sensitive-key + known-token-pattern".to_string(),
            generated_by_user_action: true,
        }
    }
}

/// Shareable diagnostics payload. Export is only produced when the user explicitly requests it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsExport {
    pub format_version: u32,
    pub generated_at: i64,
    pub report: DiagnosticReport,
    pub recent_failures: Vec<FailureSummary>,
    pub privacy: PrivacySummary,
    pub notes: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct FailureRow {
    id: String,
    chat_id: String,
    status: String,
    resume_count: i64,
    error: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
}

const REQUIRED_TABLES: [&str; 6] = [
    "projects",
    "chats",
    "messages",
    "runs",
    "run_events",
    "tool_calls",
];

/// Builds a redacted, non-sensitive diagnostic report inspecting runtime environment
/// and SQLite health. The report never includes prompts, source files, secrets, or environment
/// dumps.
pub async fn collect_diagnostics(pool: &SqlitePool, db_path: &Path) -> Result<DiagnosticReport> {
    let probe: Result<i64, sqlx::Error> = sqlx::query_scalar("SELECT 1").fetch_one(pool).await;
    let database_reachable = probe.is_ok();

    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    let fk_flag: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let applied_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let active_projects_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE archived_at IS NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let enabled_extensions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM extensions WHERE enabled = 1")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let pending_approvals_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM approvals WHERE status = 'pending'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let provider_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM providers")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let model_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM models")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let present_tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
         ('projects','chats','messages','runs','run_events','tool_calls')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let db_healthy = fk_flag == 1 && applied_migrations > 0;

    let health_checks = vec![
        HealthCheck {
            name: "database_reachable".to_string(),
            status: if database_reachable {
                HealthStatus::Ok
            } else {
                HealthStatus::Failed
            },
            detail: if database_reachable {
                "SQLite answered a `SELECT 1` probe".to_string()
            } else {
                "SQLite did not answer a read probe; check file permissions and disk space"
                    .to_string()
            },
        },
        HealthCheck {
            name: "migrations_applied".to_string(),
            status: if applied_migrations > 0 {
                HealthStatus::Ok
            } else {
                HealthStatus::Failed
            },
            detail: format!("{applied_migrations} successful migration(s) recorded"),
        },
        HealthCheck {
            name: "foreign_keys_enabled".to_string(),
            status: if fk_flag == 1 {
                HealthStatus::Ok
            } else {
                HealthStatus::Failed
            },
            detail: format!("PRAGMA foreign_keys = {fk_flag}"),
        },
        HealthCheck {
            name: "journal_mode".to_string(),
            status: if journal_mode.eq_ignore_ascii_case("wal")
                || journal_mode.eq_ignore_ascii_case("memory")
            {
                HealthStatus::Ok
            } else {
                HealthStatus::Degraded
            },
            detail: format!("PRAGMA journal_mode = {journal_mode}"),
        },
        HealthCheck {
            name: "required_tables".to_string(),
            status: if present_tables == REQUIRED_TABLES.len() as i64 {
                HealthStatus::Ok
            } else {
                HealthStatus::Failed
            },
            detail: format!(
                "{present_tables}/{} core tables present",
                REQUIRED_TABLES.len()
            ),
        },
        HealthCheck {
            name: "pending_approvals".to_string(),
            status: HealthStatus::Ok,
            detail: format!("{pending_approvals_count} approval(s) waiting for a decision"),
        },
    ];

    let chatgpt_configured = std::env::var("GAGGLE_CHATGPT_CLIENT_ID")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    let remote_state = if provider_count > 0 && model_count > 0 {
        CapabilityState::Available
    } else if provider_count > 0 {
        CapabilityState::NeedsSetup
    } else {
        CapabilityState::Unavailable
    };

    let capabilities = vec![
        CapabilityStatus::new(
            "local_mock_model",
            "Offline built-in model",
            CapabilityState::Available,
            "The deterministic `local-mock` model runs without network access.",
            None,
        ),
        CapabilityStatus::new(
            "remote_models",
            "Remote model providers",
            remote_state,
            match remote_state {
                CapabilityState::Available => format!(
                    "{provider_count} provider(s) and {model_count} model(s) configured. Credential values are never included in diagnostics."
                ),
                CapabilityState::NeedsSetup => {
                    "A provider is configured but no model is registered for it.".to_string()
                }
                _ => "No provider is configured, so only the offline built-in model is available."
                    .to_string(),
            },
            Some("Open Settings → Providers & Models to add a provider and model."),
        ),
        CapabilityStatus::new(
            "chatgpt_oauth",
            "ChatGPT OAuth sign-in",
            if chatgpt_configured {
                CapabilityState::Available
            } else {
                CapabilityState::NeedsSetup
            },
            if chatgpt_configured {
                "GAGGLE_CHATGPT_CLIENT_ID is set; create a `chatgpt` provider to sign in."
                    .to_string()
            } else {
                "GAGGLE_CHATGPT_CLIENT_ID is not set, so OAuth sign-in cannot start.".to_string()
            },
            Some(
                "Set GAGGLE_CHATGPT_CLIENT_ID and restart Gaggle, then open Settings → Providers & Models.",
            ),
        ),
        CapabilityStatus::new(
            "ollama_endpoint",
            "Local Ollama endpoint",
            CapabilityState::Unverified,
            "Ollama needs a reachable local endpoint. Diagnostics never perform network probes.",
            Some(
                "Configure an Ollama provider with http://127.0.0.1:11434 and verify it in Settings.",
            ),
        ),
        CapabilityStatus::new(
            "external_mcp_transports",
            "External MCP transports",
            CapabilityState::Unavailable,
            "External MCP transport clients are not implemented in this build; built-in extensions remain available.",
            None,
        ),
        CapabilityStatus::new(
            "diagnostics_export",
            "Redacted diagnostics export",
            CapabilityState::Available,
            "Export runs only on explicit user or CLI action and redacts secrets before serialization.",
            None,
        ),
        CapabilityStatus::new(
            "automatic_telemetry",
            "Automatic telemetry (disabled by design)",
            CapabilityState::Unavailable,
            "Gaggle does not send telemetry, crash reports, prompts, or source files to any remote service.",
            None,
        ),
    ];

    Ok(DiagnosticReport {
        service: "gaggle".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        db_path: db_path.display().to_string(),
        db_healthy,
        journal_mode,
        foreign_keys_enabled: fk_flag == 1,
        applied_migrations,
        active_projects_count,
        enabled_extensions_count,
        pending_approvals_count,
        build: BuildIdentity::current(),
        core_health: CoreHealth::from_checks(health_checks),
        capabilities,
        generated_at: now_ms(),
    })
}

/// Builds an explicit, user-initiated diagnostics export with recent redacted failures.
///
/// The export contains run identifiers, failure categories, and redacted messages. It contains no
/// prompts, source files, tool arguments, or raw tool output.
pub async fn export_diagnostics(pool: &SqlitePool, db_path: &Path) -> Result<DiagnosticsExport> {
    let report = collect_diagnostics(pool, db_path).await?;

    let failures = sqlx::query_as::<_, FailureRow>(
        r#"
        SELECT id, chat_id, status, resume_count, error, started_at, finished_at
        FROM runs
        WHERE status IN ('failed', 'paused')
        ORDER BY COALESCE(finished_at, started_at, 0) DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let recent_failures = failures
        .into_iter()
        .map(|row| {
            let raw_message = row.error.unwrap_or_else(|| "No error recorded".to_string());
            let failure_category = extract_failure_category(&raw_message);
            FailureSummary {
                run_id: row.id,
                chat_id: row.chat_id,
                status: row.status,
                failure_category,
                message: truncate_for_message(&redact_log_text(&raw_message)),
                resume_count: row.resume_count,
                started_at: row.started_at,
                finished_at: row.finished_at,
            }
        })
        .collect();

    Ok(DiagnosticsExport {
        format_version: 1,
        generated_at: now_ms(),
        report,
        recent_failures,
        privacy: PrivacySummary::default(),
        notes: vec![
            "This bundle is generated only when requested; Gaggle does not upload diagnostics automatically.".to_string(),
            "Database and workspace paths can reveal local directory names. Review before sharing.".to_string(),
            "Prompts, source files, tool arguments, and raw tool output are intentionally excluded.".to_string(),
        ],
    })
}

fn extract_failure_category(message: &str) -> Option<String> {
    message
        .split_whitespace()
        .find_map(|token| token.strip_prefix("category="))
        .map(|category| {
            category
                .trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '_'
                })
                .to_string()
        })
        .filter(|category| !category.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{BuildIdentity, CapabilityState, extract_failure_category};

    #[test]
    fn build_identity_reports_version_profile_and_target() {
        let build = BuildIdentity::current();
        assert_eq!(build.service, "gaggle");
        assert!(!build.version.is_empty());
        assert!(matches!(build.profile.as_str(), "debug" | "release"));
        assert!(build.target.contains('-'));
        assert!(!build.commit.is_empty());
    }

    #[test]
    fn failure_category_is_extracted_from_actionable_message() {
        let message = "category=provider_timeout operation=agent.turn data_preservation=preserved";
        assert_eq!(
            extract_failure_category(message).as_deref(),
            Some("provider_timeout")
        );
        assert_eq!(extract_failure_category("plain error"), None);
    }

    #[test]
    fn capability_state_serializes_as_snake_case() {
        let rendered = serde_json::to_string(&CapabilityState::NeedsSetup).expect("serialize");
        assert_eq!(rendered, "\"needs_setup\"");
    }
}
