use crate::protocol::{CreateFileArgs, CreateFileResult, EngineError, ErrorCode};
use crate::workspace::Workspace;
use serde_json::json;
use std::fs;

pub fn run(ws: &Workspace, args: CreateFileArgs) -> Result<CreateFileResult, EngineError> {
    let resolved = ws.resolve_path(&args.file_path)?;

    // Only support utf8 encoding for now
    if args.encoding.to_lowercase() != "utf8" && args.encoding.to_lowercase() != "utf-8" {
        return Err(EngineError::new(
            ErrorCode::InvalidArgument,
            "Only UTF-8 encoding is currently supported",
        )
        .with_details(json!({ "encoding": args.encoding })));
    }

    // Create parent directories if they don't exist
    if let Some(parent) = resolved.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to create parent directories")
                    .with_details(json!({
                        "path": args.file_path,
                        "io": e.to_string()
                    }))
            })?;
        }
    }

    // Check if path exists and is a directory
    if resolved.exists() && resolved.is_dir() {
        return Err(
            EngineError::new(ErrorCode::InvalidArgument, "Path exists and is a directory")
                .with_details(json!({ "path": args.file_path })),
        );
    }

    let already_exists = resolved.exists();

    // Write the file
    let bytes_written = args.contents.as_bytes().len();
    fs::write(&resolved, &args.contents).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to write file").with_details(json!({
            "path": args.file_path,
            "io": e.to_string()
        }))
    })?;

    Ok(CreateFileResult {
        file_path: args.file_path,
        created: !already_exists,
        bytes_written,
    })
}


#[cfg(test)]
#[path = "create_file_test.rs"]
mod create_file_test;
