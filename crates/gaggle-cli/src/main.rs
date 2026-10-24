use anyhow::Result;
use clap::{Parser, Subcommand};
use gaggle_core::{WorkspacePathPolicy, collect_diagnostics};
use gaggle_db::{connect_db, default_db_path, repositories::projects::ProjectRepository};
use gaggle_tools::register_builtin_extensions;
use std::{net::SocketAddr, path::PathBuf};
use tracing_subscriber::EnvFilter;

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
    Doctor,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(p) => p,
        None => default_db_path()?,
    };

    match cli.command {
        Commands::DbPath => {
            println!("{}", db_path.display());
        }
        Commands::Doctor => {
            let pool = connect_db(&db_path).await?;
            register_builtin_extensions(&pool).await?;
            let report = collect_diagnostics(&pool, &db_path).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
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
