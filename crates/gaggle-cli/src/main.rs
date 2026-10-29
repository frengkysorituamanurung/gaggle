use anyhow::Result;
use clap::{Parser, Subcommand};
use gaggle_core::{
    DiagnosticReport, HealthStatus, WorkspacePathPolicy, collect_diagnostics, init_tracing,
};
use gaggle_db::{connect_db, default_db_path, repositories::projects::ProjectRepository};
use gaggle_tools::register_builtin_extensions;
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "gaggle",
    version,
    about = "Gaggle — Self-hosted AI agent workspace"
)]
struct Cli {
    /// Custom path to gaggle.db (defaults to OS Application Support / Data dir)
    #[arg(long)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Gaggle HTTP/SSE server and web UI
    Serve {
        #[arg(long, default_value = "127.0.0.1:4321")]
        addr: SocketAddr,
    },
    /// Manage projects registered in Gaggle
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
    },
    /// Export a chat bundle or project memories
    Export {
        #[command(subcommand)]
        action: ExportCommands,
    },
    /// Import a validated chat bundle as a new chat
    Import {
        #[command(subcommand)]
        action: ImportCommands,
    },
    /// Create a checksummed backup of the database and attachments
    Backup {
        /// Destination archive path (for example backup.tar.gz)
        #[arg(long)]
        output: PathBuf,
        /// Overwrite the output archive if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Verify and restore a backup archive
    Restore {
        /// Backup archive created by `gaggle backup`
        archive: PathBuf,
        /// Replace existing database/attachments after moving them aside
        #[arg(long)]
        force: bool,
        /// Verify checksums and integrity without changing any data
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the resolved path to the global SQLite database
    DbPath,
    /// Run a redacted diagnostic health check of the environment and database
    Doctor {
        /// Emit the full machine-readable JSON export instead of the human-readable summary
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// Register a workspace directory as a Gaggle project
    Add {
        name: String,
        root_path: String,
        #[arg(long, default_value = "git")]
        vcs: String,
    },
    /// List active projects
    List,
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export one chat as a portable, versioned JSON bundle
    Chat {
        /// Chat id to export
        id: String,
        /// Destination JSON file
        #[arg(long)]
        output: PathBuf,
    },
    /// Export project memories as Markdown files for Git sharing
    Memory {
        /// Project id whose memories should be exported
        #[arg(long)]
        project: String,
        /// Destination directory (defaults to <project root>/.pal/memory)
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ImportCommands {
    /// Import a chat bundle into a project as a new chat
    Chat {
        /// Chat bundle JSON file
        file: PathBuf,
        /// Target project id
        #[arg(long)]
        project: String,
    },
    /// Import one Goose SQLite session into a project without modifying the source database
    GooseSession {
        /// Read-only path to Goose's SQLite database
        #[arg(long = "source-db")]
        source_database: PathBuf,
        /// Goose session id to import
        #[arg(long)]
        session: String,
        /// Existing Gaggle project id to receive the imported chat
        #[arg(long)]
        project: String,
        /// Attempt to copy API keys from Goose's OS credential vault into Gaggle's
        #[arg(long = "import-goose-credentials")]
        import_credentials: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(p) => p,
        None => default_db_path()?,
    };

    match cli.command {
        Commands::DbPath => {
            println!("{}", db_path.display());
        }
        Commands::Doctor { json } => {
            let pool = connect_db(&db_path).await?;
            register_builtin_extensions(&pool).await?;
            let report = collect_diagnostics(&pool, &db_path).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", format_doctor_report(&report));
            }
            if report.core_health.status == HealthStatus::Failed {
                anyhow::bail!(
                    "Gaggle doctor found a failed core health check. Review the report above, then retry."
                );
            }
        }
        Commands::Serve { addr } => {
            let pool = connect_db(&db_path).await?;
            gaggle_server::serve(pool, addr).await?;
        }
        Commands::Project { action } => {
            let pool = connect_db(&db_path).await?;
            let repo = ProjectRepository::new(pool);
            match action {
                ProjectCommands::Add {
                    name,
                    root_path,
                    vcs,
                } => {
                    let policy = WorkspacePathPolicy::new(&root_path)?;
                    let canonical_root = policy.canonical_root().to_string_lossy().to_string();
                    let proj = repo.create(&name, &canonical_root, &vcs).await?;
                    println!("{}", serde_json::to_string_pretty(&proj)?);
                }
                ProjectCommands::List => {
                    let list = repo.list_active().await?;
                    println!("{}", serde_json::to_string_pretty(&list)?);
                }
            }
        }
        Commands::Export { action } => {
            let pool = connect_db(&db_path).await?;
            match action {
                ExportCommands::Chat { id, output } => {
                    let summary = gaggle_cli::chat_bundle::export_chat(&pool, &id, &output).await?;
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
                ExportCommands::Memory { project, output } => {
                    let report = gaggle_cli::memory_export::export_memories(
                        &pool,
                        &project,
                        output.as_deref(),
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        Commands::Import { action } => {
            let pool = connect_db(&db_path).await?;
            match action {
                ImportCommands::Chat { file, project } => {
                    let summary =
                        gaggle_cli::chat_bundle::import_chat(&pool, &project, &file).await?;
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
                ImportCommands::GooseSession {
                    source_database,
                    session,
                    project,
                    import_credentials,
                } => {
                    let summary = gaggle_cli::goose_import::import_goose_session_with_credentials(
                        &pool,
                        &source_database,
                        &session,
                        &project,
                        import_credentials,
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
        }
        Commands::Backup { output, force } => {
            // Backups intentionally do not open the application pool or run
            // migrations: the archive must reflect the database as it exists.
            let report = gaggle_cli::backup::create_backup(&db_path, &output, force).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Restore {
            archive,
            force,
            dry_run,
        } => {
            let report =
                gaggle_cli::backup::restore_backup(&archive, &db_path, force, dry_run).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

/// Renders a human-readable `gaggle doctor` summary.
///
/// The summary reports build identity, core health, optional capability availability, and the
/// privacy posture. It never includes prompts, source files, secrets, or raw tool output.
fn format_doctor_report(report: &DiagnosticReport) -> String {
    let mut output = String::new();
    output.push_str("Gaggle doctor\n");
    output.push_str(&format!(
        "Build: {} {} ({}, {}, commit {})\n",
        report.build.service,
        report.build.version,
        report.build.profile,
        report.build.target,
        report.build.commit
    ));
    output.push_str(&format!("Rust compiler: {}\n", report.build.rustc));
    output.push_str(&format!("Database: {}\n", report.db_path));
    output.push_str(&format!(
        "SQLite: journal_mode={}, foreign_keys={}, migrations={}\n",
        report.journal_mode,
        if report.foreign_keys_enabled {
            "on"
        } else {
            "off"
        },
        report.applied_migrations
    ));
    output.push_str(&format!(
        "Core health: {}\n",
        report.core_health.status.as_str()
    ));
    for check in &report.core_health.checks {
        output.push_str(&format!(
            "  {} {}: {}\n",
            status_marker(check.status),
            check.name,
            check.detail
        ));
    }

    output.push_str("Optional capabilities:\n");
    for capability in &report.capabilities {
        output.push_str(&format!(
            "  {} {} [{}]: {}\n",
            status_marker_for_capability(capability.available, capability.state),
            capability.label,
            capability.state.as_str(),
            capability.reason
        ));
        if !capability.available
            && let Some(action) = &capability.action
        {
            output.push_str(&format!("      action: {action}\n"));
        }
    }

    output.push_str(&format!(
        "Counts: projects={}, enabled_extensions={}, pending_approvals={}\n",
        report.active_projects_count,
        report.enabled_extensions_count,
        report.pending_approvals_count
    ));
    output.push_str(
        "Privacy: no telemetry, no crash reporting, no prompts or source files in logs. \
         Diagnostics export is explicit (Settings → Diagnostics or `gaggle doctor --json`).\n",
    );
    output.push_str(
        "Next step: if a check failed, fix the reported action and rerun `gaggle doctor`.\n",
    );
    output
}

fn status_marker(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Ok => "✓",
        HealthStatus::Degraded => "!",
        HealthStatus::Failed => "✗",
    }
}

fn status_marker_for_capability(
    available: bool,
    state: gaggle_core::CapabilityState,
) -> &'static str {
    if available {
        "✓"
    } else if state == gaggle_core::CapabilityState::Unverified {
        "?"
    } else {
        "✗"
    }
}

#[cfg(test)]
mod tests {
    use super::format_doctor_report;
    use gaggle_core::{
        BuildIdentity, CapabilityState, CapabilityStatus, CoreHealth, DiagnosticReport,
        HealthCheck, HealthStatus,
    };

    fn sample_report() -> DiagnosticReport {
        DiagnosticReport {
            service: "gaggle".to_string(),
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            db_path: "/tmp/gaggle.db".to_string(),
            db_healthy: true,
            journal_mode: "wal".to_string(),
            foreign_keys_enabled: true,
            applied_migrations: 1,
            active_projects_count: 2,
            enabled_extensions_count: 5,
            pending_approvals_count: 1,
            build: BuildIdentity {
                service: "gaggle".to_string(),
                version: "0.1.0".to_string(),
                commit: "abc1234".to_string(),
                rustc: "rustc 1.90.0".to_string(),
                profile: "debug".to_string(),
                target: "aarch64-macos".to_string(),
            },
            core_health: CoreHealth {
                status: HealthStatus::Degraded,
                checks: vec![HealthCheck {
                    name: "journal_mode".to_string(),
                    status: HealthStatus::Degraded,
                    detail: "PRAGMA journal_mode = delete".to_string(),
                }],
            },
            capabilities: vec![
                CapabilityStatus {
                    id: "remote_models".to_string(),
                    label: "Remote model providers".to_string(),
                    available: false,
                    state: CapabilityState::Unavailable,
                    reason: "No provider is configured.".to_string(),
                    action: Some("Add a provider in Settings.".to_string()),
                },
                CapabilityStatus {
                    id: "ollama_endpoint".to_string(),
                    label: "Local Ollama endpoint".to_string(),
                    available: false,
                    state: CapabilityState::Unverified,
                    reason: "Diagnostics never probe the network.".to_string(),
                    action: None,
                },
            ],
            generated_at: 1,
        }
    }

    #[test]
    fn doctor_output_reports_build_health_and_capability_reasons() {
        let text = format_doctor_report(&sample_report());

        assert!(text.contains("Gaggle doctor"));
        assert!(text.contains("Build: gaggle 0.1.0 (debug, aarch64-macos, commit abc1234)"));
        assert!(text.contains("Core health: degraded"));
        assert!(text.contains("! journal_mode"));
        assert!(text.contains("Optional capabilities:"));
        assert!(text.contains("Remote model providers [unavailable]: No provider is configured."));
        assert!(text.contains("action: Add a provider in Settings."));
        assert!(text.contains("Local Ollama endpoint [unverified]"));
        assert!(text.contains("no telemetry"));
    }
}
