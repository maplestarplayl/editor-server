use crate::rpc::error::{
    DIRECTORY_ERROR_CODE, FILE_NOT_FOUND_CODE, INVALID_PARAMS_CODE, IO_ERROR_CODE,
    METHOD_NOT_FOUND_CODE,
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

#[derive(Deserialize)]
struct ListFilesParams {
    path: String,
}

#[derive(Debug)]
enum HandlerError {
    InvalidParams(String),
    FileNotFound,
    DirectoryError(String),
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
            HandlerError::DirectoryError(msg) => {
                error!(error_type = "directory_error", message = %msg, "Request failed");
                create_error_response(DIRECTORY_ERROR_CODE, msg, id)
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
        "listFiles" => {
            debug!("Handling listFiles request");
            handle_list_files(request.params)
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

fn handle_list_files(params: Value) -> Result<Value, HandlerError> {
    let file_span = info_span!("list_files_operation");
    let _enter = file_span.enter();

    let params: ListFilesParams = serde_json::from_value(params).map_err(|e| {
        debug!(error = %e, "Failed to deserialize list files parameters");
        HandlerError::InvalidParams(e.to_string())
    })?;

    debug!(path = %params.path, "Listing files in directory");
    let path = Path::new(&params.path);

    if !path.exists() {
        debug!(path = %params.path, "Directory does not exist");
        return Err(HandlerError::DirectoryError(
            "Directory does not exist".to_string(),
        ));
    }

    if !path.is_dir() {
        debug!(path = %params.path, "Path is not a directory");
        return Err(HandlerError::DirectoryError(
            "Path is not a directory".to_string(),
        ));
    }

    let entries = fs::read_dir(path).map_err(|e| {
        debug!(path = %params.path, error = %e, "Failed to read directory");
        HandlerError::IoError(e)
    })?;

    let mut files = Vec::new();
    let mut directories = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| {
            debug!(path = %params.path, error = %e, "Failed to read directory entry");
            HandlerError::IoError(e)
        })?;

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            directories.push(serde_json::json!({
                "name": name,
                "type": "directory"
            }));
        } else {
            let metadata = entry.metadata().map_err(|e| {
                debug!(path = %path.display(), error = %e, "Failed to read file metadata");
                HandlerError::IoError(e)
            })?;

            files.push(serde_json::json!({
                "name": name,
                "type": "file",
                "size": metadata.len()
            }));
        }
    }

    // Sort directories first, then files, both alphabetically
    directories.sort_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()));
    files.sort_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()));

    let mut result = directories;
    result.extend(files);

    info!(
        path = %params.path,
        total_items = result.len(),
        "Directory listing completed successfully"
    );

    Ok(Value::Array(result))
}
