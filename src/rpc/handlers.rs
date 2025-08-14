use crate::rpc::error::{FILE_NOT_FOUND_CODE, INVALID_PARAMS_CODE, IO_ERROR_CODE, METHOD_NOT_FOUND_CODE};

use super::error::create_error_response;
use super::request::{JsonRpcRequest, JsonRpcResponse};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, io::Write, path::Path};
use tracing::{info, warn};
#[derive(Deserialize)]
struct ReadFileParams {
    path: String,
}

#[derive(Deserialize)]
struct WriteFileParams {
    path: String,
    content: String,
}

#[derive(Debug)]
enum HandlerError {
    InvalidParams(String),
    FileNotFound,
    IoError(std::io::Error),
}
impl HandlerError {
    fn to_jsonrpc_error(&self, id: Value) -> JsonRpcResponse {
        match self {
            HandlerError::InvalidParams(msg) => {
                warn!("Invalid parameters: {}", msg);
                create_error_response(INVALID_PARAMS_CODE, msg, id)
            }
            HandlerError::FileNotFound => {
                warn!("File not found");
                create_error_response(FILE_NOT_FOUND_CODE, "File not found", id)
            }
            HandlerError::IoError(e) => {
                warn!("IO error: {}", e);
                create_error_response(IO_ERROR_CODE, &e.to_string(), id)
            }
        }
    }
}

pub fn process_request(request: JsonRpcRequest) -> JsonRpcResponse {
    info!("Processing request: method={}, id={:?}", request.method, request.id);

    let id = request.id.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        "readFile" => handle_read_file(request.params),
        "writeFile" => handle_write_file(request.params),
        _ => {
            warn!("Unknown method: {}", request.method);
            return create_error_response(METHOD_NOT_FOUND_CODE, "Method not Found", id)
        }
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id,
        },
        Err(e) => e.to_jsonrpc_error(id),
    }


}

fn handle_read_file(params: Value) -> Result<Value, HandlerError> {
    let params: ReadFileParams = serde_json::from_value(params)
        .map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let path = Path::new(&params.path);

    if !path.exists() {
        return Err(HandlerError::FileNotFound);
    }

    let content = fs::read_to_string(path)
        .map_err(HandlerError::IoError)?;
    
    info!("Successfully Read file {}: {}", params.path, content);
    Ok(Value::String(content))
}

fn handle_write_file(params: Value) -> Result<Value, HandlerError> {
    let params: WriteFileParams = serde_json::from_value(params)
        .map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let path = Path::new(&params.path);

    let mut file = fs::File::create(path)
        .map_err(HandlerError::IoError)?;

    file.write_all(params.content.as_bytes())
        .map_err(HandlerError::IoError)?;

    info!("Successfully wrote file {}: {}", params.path, params.content);
    Ok(Value::Bool(true))
}
