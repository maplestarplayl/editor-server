mod rpc;
mod state;
mod ws;

use axum::{Router, routing::get};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tracing::{error, info, info_span};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    let server_span = info_span!("editor_server", version = "0.1.3");
    let _enter = server_span.enter();

    const SERVER_ADDRESS: ([u8; 4], u16) = ([0, 0, 0, 0], 3000); //TODO: maybe should only listen container addr

    let state: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(SERVER_ADDRESS);
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!(address = %addr, "Server starting");

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap_or_else(|e| error!(error = %e, "Server error"));
}
