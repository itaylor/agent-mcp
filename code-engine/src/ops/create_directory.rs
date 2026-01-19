use crate::protocol::{CreateDirectoryArgs, CreateDirectoryResult, EngineError, ErrorCode};
use crate::workspace::Workspace;
use serde_json::json;
use std::fs;

pub fn run(
    ws: &Workspace,
    args: CreateDirectoryArgs,
) -> Result<CreateDirectoryResult, EngineError> {
    let resolved = ws.resolve_path(&args.dir_path)?;

    // Check if it already exists
    let already_exists = resolved.exists();

    if already_exists {
        // If it exists and is a directory, consider this success
        if resolved.is_dir() {
            return Ok(CreateDirectoryResult {
                dir_path: args.dir_path,
                created: false, // Already existed
            });
        } else {
            // Exists but is not a directory
            return Err(EngineError::new(
                ErrorCode::InvalidArgument,
                "Path exists but is not a directory",
            )
            .with_details(json!({ "path": args.dir_path })));
        }
    }

    // Create the directory and all parent directories
    fs::create_dir_all(&resolved).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to create directory").with_details(json!({
            "path": args.dir_path,
            "io": e.to_string()
        }))
    })?;

    Ok(CreateDirectoryResult {
        dir_path: args.dir_path,
        created: true,
    })
}

#[cfg(test)]
#[path = "create_directory_test.rs"]
mod create_directory_test;
