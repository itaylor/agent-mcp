use crate::protocol::{EngineError, ErrorCode, ListDirArgs, ListDirEntry, ListDirResult};
use crate::workspace::Workspace;
use serde_json::json;
use walkdir::WalkDir;

pub fn run(ws: &Workspace, args: ListDirArgs) -> Result<ListDirResult, EngineError> {
    let resolved = ws.resolve_path(&args.dir_path)?;

    if !resolved.exists() {
        return Err(EngineError::new(ErrorCode::NotFound, "Directory not found")
            .with_details(json!({ "path": args.dir_path })));
    }

    if !resolved.is_dir() {
        return Err(
            EngineError::new(ErrorCode::InvalidArgument, "Path is not a directory")
                .with_details(json!({ "path": args.dir_path })),
        );
    }

    let mut entries = Vec::new();
    let mut truncated = false;

    let walker = WalkDir::new(&resolved)
        .max_depth(args.depth)
        .follow_links(false);

    for entry in walker {
        if entries.len() >= args.max_entries {
            truncated = true;
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Skip entries we can't read
        };

        let file_name = entry.file_name().to_string_lossy();

        // Skip hidden files if not requested
        if !args.include_hidden && file_name.starts_with('.') && entry.depth() > 0 {
            continue;
        }

        let path = entry.path();
        let relative = path
            .strip_prefix(&resolved)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Skip the root directory itself
        if relative.is_empty() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_type = if metadata.is_dir() { "dir" } else { "file" };

        let size = if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        };

        entries.push(ListDirEntry {
            path: relative,
            entry_type: entry_type.to_string(),
            size,
        });
    }

    Ok(ListDirResult { entries, truncated })
}

#[cfg(test)]
#[path = "list_dir_test.rs"]
mod list_dir_test;
