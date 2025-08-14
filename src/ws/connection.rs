use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::rpc::{error::PARSE_ERROR_CODE, handlers::process_request};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<()>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, _state: Arc<Mutex<()>>) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(msg_result) = receiver.next().await {
        let msg = match msg_result {
            Ok(msg) => msg,
            Err(_) => return, // Connection error, close gracefully
        };

        if let Message::Text(text) = msg {
            let response = match serde_json::from_str(&text) {
                Ok(request) => process_request(request),
                Err(_) => crate::rpc::error::create_error_response(
                    PARSE_ERROR_CODE,
                    "Parse error",
                    serde_json::Value::Null,
                ),
            };

            let response_text = match serde_json::to_string(&response) {
                Ok(text) => text,
                Err(_) => continue, // Skip if we can't serialize the response
            };

            if sender
                .send(Message::Text(response_text.into()))
                .await
                .is_err()
            {
                return; // Connection closed
            }
        }
    }
}
