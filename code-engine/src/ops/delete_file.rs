use crate::protocol::{DeleteFileArgs, DeleteFileResult, EngineError, ErrorCode};
use crate::workspace::Workspace;
use serde_json::json;
use std::fs;

pub fn run(ws: &Workspace, args: DeleteFileArgs) -> Result<DeleteFileResult, EngineError> {
    let resolved = ws.resolve_path(&args.file_path)?;

    // Check if path exists
    if !resolved.exists() {
        return Err(
            EngineError::new(ErrorCode::NotFound, "File or directory not found")
                .with_details(json!({ "path": args.file_path })),
        );
    }

    // Delete based on type
    let metadata = fs::metadata(&resolved).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to read file metadata").with_details(json!({
            "path": args.file_path,
            "io": e.to_string()
        }))
    })?;

    if metadata.is_dir() {
        // Remove directory recursively
        fs::remove_dir_all(&resolved).map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to delete directory").with_details(json!({
                "path": args.file_path,
                "io": e.to_string()
            }))
        })?;
    } else {
        // Remove file
        fs::remove_file(&resolved).map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to delete file").with_details(json!({
                "path": args.file_path,
                "io": e.to_string()
            }))
        })?;
    }

    Ok(DeleteFileResult {
        file_path: args.file_path,
        deleted: true,
    })
}


#[cfg(test)]
#[path = "delete_file_test.rs"]
mod delete_file_test;
