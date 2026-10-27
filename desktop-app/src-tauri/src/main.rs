use gaggle_db::{connect_db, default_db_path};
use gaggle_server::{build_router, initialize_server_state};
use std::net::SocketAddr;
use tauri::Manager;

#[tokio::main]
async fn main() {
    let db_path = default_db_path().expect("failed to resolve db path");
    let pool = connect_db(&db_path).await.expect("failed to connect db");
    let state = initialize_server_state(pool).await.expect("failed to init state");
    let addr: SocketAddr = "127.0.0.1:4321".parse().unwrap();

    // Start axum server in background
    tokio::spawn(async move {
        let app = build_router(state);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Give server a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let _ = window.navigate("http://127.0.0.1:4321".parse().unwrap());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}