use crate::protocol::{EngineError, ErrorCode, ReadFileInfoArgs, ReadFileInfoResult};
use crate::workspace::Workspace;
use serde_json::json;
use std::fs;
use std::time::UNIX_EPOCH;

pub fn run(ws: &Workspace, args: ReadFileInfoArgs) -> Result<ReadFileInfoResult, EngineError> {
    let resolved = ws.resolve_path(&args.file_path)?;

    // Check if path exists
    if !resolved.exists() {
        return Ok(ReadFileInfoResult {
            file_path: args.file_path,
            exists: false,
            entry_type: "unknown".to_string(),
            size: None,
            mtime_ns: None,
        });
    }

    // Get metadata
    let metadata = fs::metadata(&resolved).map_err(|e| {
        EngineError::new(ErrorCode::IoError, "Failed to read file metadata").with_details(json!({
            "path": args.file_path,
            "io": e.to_string()
        }))
    })?;

    // Determine entry type
    let entry_type = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "dir"
    } else if metadata.is_symlink() {
        "symlink"
    } else {
        "other"
    };

    // Get size
    let size = if metadata.is_file() {
        Some(metadata.len())
    } else {
        None
    };

    // Get mtime
    let mtime_ns = metadata.modified().ok().and_then(|modified| {
        modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| (d.as_secs() as i128) * 1_000_000_000i128 + (d.subsec_nanos() as i128))
    });

    Ok(ReadFileInfoResult {
        file_path: args.file_path,
        exists: true,
        entry_type: entry_type.to_string(),
        size,
        mtime_ns,
    })
}

#[cfg(test)]
#[path = "read_file_info_test.rs"]
mod read_file_info_test;
