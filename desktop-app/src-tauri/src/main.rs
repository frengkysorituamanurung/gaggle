use gaggle_db::{connect_db, default_db_path};
use gaggle_server::{build_router, initialize_server_state_with_os_credentials};
use std::{io, net::SocketAddr};
use tauri::{Manager, RunEvent};
use tokio::{net::TcpListener, sync::oneshot};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = default_db_path()?;
    let pool = connect_db(&db_path).await?;
    let state = initialize_server_state_with_os_credentials(pool).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr: SocketAddr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        let router = build_router(state);
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let app = tauri::Builder::default()
        .setup(move |app| {
            let window = app.get_webview_window("main").ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Gaggle main window is missing")
            })?;
            window.navigate(format!("http://{addr}").parse()?)?;
            Ok(())
        })
        .build(tauri::generate_context!())?;

    let mut shutdown_tx = Some(shutdown_tx);
    app.run(move |_, event| {
        if matches!(event, RunEvent::Exit) {
            if let Some(shutdown_tx) = shutdown_tx.take() {
                let _ = shutdown_tx.send(());
            }
        }
    });

    server.await??;
    Ok(())
}
