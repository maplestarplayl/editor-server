use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub fn create_error_response(
    code: i32,
    message: &str,
    id: serde_json::Value,
) -> super::request::JsonRpcResponse {
    super::request::JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
        }),
        id,
    }
}

// JSON-RPC error codes
pub const PARSE_ERROR_CODE: i32 = -32700;
#[allow(dead_code)]
pub const INVALID_REQUEST_CODE: i32 = -32600;
pub const METHOD_NOT_FOUND_CODE: i32 = -32601;
pub const INVALID_PARAMS_CODE: i32 = -32602;
#[allow(dead_code)]
pub const INTERNAL_ERROR_CODE: i32 = -32603;
// Application-specific error codes
pub const FILE_NOT_FOUND_CODE: i32 = -32001;
pub const IO_ERROR_CODE: i32 = -32002;
