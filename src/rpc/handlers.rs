use crate::rpc::error::{
    FILE_NOT_FOUND_CODE, INVALID_PARAMS_CODE, IO_ERROR_CODE, METHOD_NOT_FOUND_CODE,
};

use super::error::create_error_response;
use super::request::{JsonRpcRequest, JsonRpcResponse};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, io::Write, path::Path};
use tracing::{debug, error, info, info_span, warn};
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
                error!(error_type = "invalid_params", message = %msg, "Request failed");
                create_error_response(INVALID_PARAMS_CODE, msg, id)
            }
            HandlerError::FileNotFound => {
                error!(error_type = "file_not_found", "Request failed");
                create_error_response(FILE_NOT_FOUND_CODE, "File not found", id)
            }
            HandlerError::IoError(e) => {
                error!(error_type = "io_error", error = %e, "Request failed");
                create_error_response(IO_ERROR_CODE, &e.to_string(), id)
            }
        }
    }
}

pub fn process_request(request: JsonRpcRequest) -> JsonRpcResponse {
    let method = &request.method;
    let request_id = request
        .id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "null".to_string());

    let span = info_span!(
        "rpc_request",
        method = %method,
        request_id = %request_id,
        has_params = !request.params.is_null()
    );
    let _enter = span.enter();

    info!("Processing JSON-RPC request");

    let id = request.id.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        "readFile" => {
            debug!("Handling readFile request");
            handle_read_file(request.params)
        }
        "writeFile" => {
            debug!("Handling writeFile request");
            handle_write_file(request.params)
        }
        _ => {
            warn!(method = %request.method, "Unknown method requested");
            return create_error_response(METHOD_NOT_FOUND_CODE, "Method not Found", id);
        }
    };

    match result {
        Ok(value) => {
            info!("Request processed successfully");
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(value),
                error: None,
                id,
            }
        }
        Err(e) => e.to_jsonrpc_error(id),
    }
}

fn handle_read_file(params: Value) -> Result<Value, HandlerError> {
    let file_span = info_span!("read_file_operation");
    let _enter = file_span.enter();

    let params: ReadFileParams = serde_json::from_value(params).map_err(|e| {
        debug!(error = %e, "Failed to deserialize read file parameters");
        HandlerError::InvalidParams(e.to_string())
    })?;

    debug!(path = %params.path, "Reading file");
    let path = Path::new(&params.path);

    if !path.exists() {
        debug!(path = %params.path, "File does not exist");
        return Err(HandlerError::FileNotFound);
    }

    let content = fs::read_to_string(path).map_err(|e| {
        debug!(path = %params.path, error = %e, "Failed to read file content");
        HandlerError::IoError(e)
    })?;

    info!(
        path = %params.path,
        content_length = content.len(),
        "File read successfully"
    );
    Ok(Value::String(content))
}

fn handle_write_file(params: Value) -> Result<Value, HandlerError> {
    let file_span = info_span!("write_file_operation");
    let _enter = file_span.enter();

    let params: WriteFileParams = serde_json::from_value(params).map_err(|e| {
        debug!(error = %e, "Failed to deserialize write file parameters");
        HandlerError::InvalidParams(e.to_string())
    })?;

    debug!(
        path = %params.path,
        content_length = params.content.len(),
        "Writing file"
    );
    let path = Path::new(&params.path);

    let mut file = fs::File::create(path).map_err(|e| {
        debug!(path = %params.path, error = %e, "Failed to create file");
        HandlerError::IoError(e)
    })?;

    file.write_all(params.content.as_bytes()).map_err(|e| {
        debug!(path = %params.path, error = %e, "Failed to write file content");
        HandlerError::IoError(e)
    })?;

    info!(
        path = %params.path,
        content_length = params.content.len(),
        "File written successfully"
    );
    Ok(Value::Bool(true))
}
