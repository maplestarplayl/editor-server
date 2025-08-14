mod rpc;
mod state;
mod ws;

use axum::{Router, routing::get};
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    const SERVER_ADDRESS: ([u8; 4], u16) = ([127, 0, 0, 1], 3000);

    let state: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(SERVER_ADDRESS);
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("Starting server on {}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap_or_else(|e| error!("Server error: {}", e));
}
