use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::Mutex;
use tracing::{Instrument, debug, error, info, info_span, warn};

use crate::rpc::{error::PARSE_ERROR_CODE, handlers::process_request};

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<()>>>,
) -> impl IntoResponse {
    let connection_id = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    info!(
        connection_id = connection_id,
        "WebSocket connection request received"
    );
    ws.on_upgrade(move |socket| {
        let connection_span = info_span!("ws_connection", connection_id = connection_id);
        handle_socket(socket, state, connection_id).instrument(connection_span)
    })
}

async fn handle_socket(socket: WebSocket, _state: Arc<Mutex<()>>, connection_id: u64) {
    info!(
        connection_id = connection_id,
        "WebSocket connection established"
    );
    let (mut sender, mut receiver) = socket.split();

    while let Some(msg_result) = receiver.next().await {
        let msg = match msg_result {
            Ok(msg) => msg,
            Err(e) => {
                warn!(connection_id = connection_id, error = %e, "WebSocket message error");
                return; // Connection error, close gracefully
            }
        };

        if let Message::Text(text) = msg {
            let request_span = info_span!(
                "process_request",
                connection_id = connection_id,
                request_size = text.len()
            );
            let _enter = request_span.enter();

            debug!(request = %text, "Received JSON-RPC request");

            let response = match serde_json::from_str(&text) {
                Ok(request) => {
                    debug!("Request parsed successfully");
                    process_request(request)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse JSON-RPC request");
                    crate::rpc::error::create_error_response(
                        PARSE_ERROR_CODE,
                        "Parse error",
                        serde_json::Value::Null,
                    )
                }
            };

            let response_text = match serde_json::to_string(&response) {
                Ok(text) => {
                    debug!(
                        response_size = text.len(),
                        "Response serialized successfully"
                    );
                    text
                }
                Err(e) => {
                    error!(error = %e, "Failed to serialize response");
                    continue; // Skip if we can't serialize the response
                }
            };

            if let Err(e) = sender.send(Message::Text(response_text.into())).await {
                warn!(connection_id = connection_id, error = %e, "Failed to send response");
                return; // Connection closed
            }

            debug!("Response sent successfully");
        }
    }

    info!(connection_id = connection_id, "WebSocket connection closed");
}
