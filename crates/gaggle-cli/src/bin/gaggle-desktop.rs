use anyhow::{Context, Result, bail};
use clap::Parser;
use gaggle_db::{connect_db, default_db_path};
use std::{net::SocketAddr, path::PathBuf, process::Command, time::Duration};
use tracing::{info, warn};

/// Desktop entry point for Gaggle.
///
/// Gaggle's UI is served by the local Axum server rather than an embedded
/// desktop webview. This binary keeps the server in the foreground and opens
/// the default browser once the health endpoint is reachable.
#[derive(Parser)]
#[command(
    name = "gaggle-desktop",
    version,
    about = "Launch the Gaggle local server and web UI"
)]
struct DesktopCli {
    /// Custom path to gaggle.db (defaults to OS Application Support / Data dir)
    #[arg(long)]
    db: Option<PathBuf>,

    /// Local address for the Gaggle server
    #[arg(long, default_value = "127.0.0.1:4321")]
    addr: SocketAddr,

    /// Do not open the default browser after the server starts
    #[arg(long)]
    no_browser: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    gaggle_core::init_tracing()?;

    let cli = DesktopCli::parse();
    let db_path = match cli.db {
        Some(path) => path,
        None => default_db_path()?,
    };

    let pool = connect_db(&db_path).await?;
    let addr = cli.addr;

    if !cli.no_browser {
        tokio::spawn(async move {
            if wait_for_server(addr).await {
                if let Err(error) = open_browser(addr) {
                    warn!(%error, "Could not open the Gaggle web UI in a browser");
                }
            } else {
                warn!(%addr, "Gaggle server did not become ready; not opening a browser");
            }
        });
    }

    info!(db_path = %db_path.display(), %addr, "Starting Gaggle desktop");
    gaggle_server::serve(pool, addr).await?;
    Ok(())
}

async fn wait_for_server(addr: SocketAddr) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/api/health");
    for _ in 0..100 {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                if let Ok(body) = response.text().await {
                    if body.contains("\"status\":\"ok\"") {
                        return true;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

fn browser_url(addr: SocketAddr) -> String {
    format!("http://{addr}")
}

fn open_browser(addr: SocketAddr) -> Result<()> {
    let url = browser_url(addr);

    // Prefer launching a Chromium-based browser in application mode so Gaggle
    // behaves like an installed desktop app (dedicated window, no browser chrome)
    // instead of a plain browser tab. Fall back to the platform default opener.
    let app_mode_candidates: &[&str] = match std::env::consts::OS {
        "macos" => &[
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "google-chrome",
            "chromium",
            "microsoft-edge",
        ],
        "linux" => &[
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "microsoft-edge-stable",
        ],
        _ => &[],
    };

    for candidate in app_mode_candidates {
        if Command::new(candidate)
            .arg(format!("--app={url}"))
            .arg("--new-window")
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }

    let program = match std::env::consts::OS {
        "macos" => "open",
        "linux" => "xdg-open",
        other => bail!("opening a browser is not supported on {other}"),
    };

    let status = Command::new(program)
        .arg(&url)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if !status.success() {
        bail!("{program} exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::browser_url;

    #[test]
    fn browser_url_formats_ipv4_and_ipv6_addresses() {
        assert_eq!(
            browser_url("127.0.0.1:4321".parse().expect("valid IPv4 address")),
            "http://127.0.0.1:4321"
        );
        assert_eq!(
            browser_url("[::1]:4321".parse().expect("valid IPv6 address")),
            "http://[::1]:4321"
        );
    }
}
