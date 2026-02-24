use crate::protocol::{EngineError, ErrorCode, ReadFileArgs, ReadFileResult};
use crate::workspace::Workspace;
use serde_json::json;
use std::fs;

const MAX_FILE_BYTES: usize = 102_400; // 100KB hard limit

pub fn run(ws: &Workspace, args: ReadFileArgs) -> Result<ReadFileResult, EngineError> {
    // Enforce hard limit - cap user's max_bytes to our maximum
    let max_bytes = args.max_bytes.min(MAX_FILE_BYTES);

    let resolved = ws.resolve_path(&args.file_path)?;

    // Check if file exists
    if !resolved.exists() {
        return Err(EngineError::new(ErrorCode::NotFound, "File not found")
            .with_details(json!({ "path": args.file_path })));
    }

    // Check if it's a directory
    if resolved.is_dir() {
        return Err(EngineError::new(
            ErrorCode::InvalidArgument,
            "Path is a directory, not a file",
        )
        .with_details(json!({ "path": args.file_path })));
    }

    // Get file size
    let metadata = fs::metadata(&resolved).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to read file metadata").with_details(json!({
            "path": args.file_path,
            "io": e.to_string()
        }))
    })?;

    let file_size = metadata.len();

    // Check size limit (hard limit of 100KB)
    if file_size > max_bytes as u64 {
        return Err(EngineError::new(
            ErrorCode::InvalidArgument,
            format!(
                "File size ({} bytes) exceeds maximum allowed ({} bytes, hard limit 100KB). Use read_excerpt to read parts of large files.",
                file_size, max_bytes
            ),
        )
        .with_details(json!({
            "path": args.file_path,
            "size": file_size,
            "maxBytes": max_bytes,
            "hardLimit": MAX_FILE_BYTES
        })));
    }

    // Read the file as UTF-8
    let contents = fs::read_to_string(&resolved).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to read file as UTF-8").with_details(json!({
            "path": args.file_path,
            "io": e.to_string()
        }))
    })?;

    Ok(ReadFileResult {
        file_path: args.file_path,
        contents,
        size: file_size,
        truncated: false, // We error on oversized files, so never truncated
    })
}

#[cfg(test)]
#[path = "read_file_test.rs"]
mod read_file_test;
