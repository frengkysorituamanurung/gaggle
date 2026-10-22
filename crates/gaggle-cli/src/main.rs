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
    }

    Ok(())
}
