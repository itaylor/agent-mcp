use crate::protocol::{EngineError, ErrorCode};
use crate::workspace::Workspace;
use ropey::Rope;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMeta {
    pub size: u64,
    pub mtime_ns: i128,
}

#[derive(Debug)]
pub struct CacheEntry {
    pub meta: FileMeta,
    pub rope: Rope,
}

#[derive(Default)]
pub struct RopeCache {
    // Keyed by canonical absolute path.
    // TODO: This cache grows unbounded and will consume unlimited memory in long-running
    // sessions that access hundreds of files. Need to implement LRU eviction with a
    // configurable size limit (e.g., max entries or max total bytes).
    entries: HashMap<PathBuf, CacheEntry>,
}

impl RopeCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a "fresh" rope for a file:
    /// - resolve path within repo root
    /// - stat and compare (mtime,size)
    /// - if changed or not cached -> reload from disk, replace cache entry
    pub fn get_fresh_rope(
        &mut self,
        ws: &Workspace,
        user_path: &str,
    ) -> Result<(PathBuf, &Rope), EngineError> {
        let resolved = ws.resolve_path(user_path)?;

        let meta = std::fs::metadata(&resolved).map_err(|e| {
            EngineError::new(ErrorCode::NotFound, "File not found")
                .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;

        let fresh_meta = file_meta(&meta).map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to read file metadata")
                .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;

        let needs_reload = match self.entries.get(&resolved) {
            None => true,
            Some(entry) => entry.meta != fresh_meta,
        };

        if needs_reload {
            // Read UTF-8 text
            let text = std::fs::read_to_string(&resolved).map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to read file as UTF-8 text")
                    .with_details(json!({ "path": user_path, "io": e.to_string() }))
            })?;

            let rope = Rope::from_str(&text);
            self.entries.insert(
                resolved.clone(),
                CacheEntry {
                    meta: fresh_meta,
                    rope,
                },
            );
        }

        // Safe unwrap since we ensured it exists above
        let entry = self.entries.get(&resolved).unwrap();
        Ok((resolved, &entry.rope))
    }

    /// After your `apply_patch` writes the file, you can optionally call this
    /// to update the cache with the new rope and new metadata (pattern for later).
    pub fn put_rope(
        &mut self,
        ws: &Workspace,
        user_path: &str,
        rope: Rope,
    ) -> Result<(), EngineError> {
        let resolved = ws.resolve_path(user_path)?;

        let meta = std::fs::metadata(&resolved).map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to stat file after write")
                .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;
        let fresh_meta = file_meta(&meta).map_err(|e| {
            EngineError::new(
                ErrorCode::IoError,
                "Failed to read file metadata after write",
            )
            .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;

        self.entries.insert(
            resolved,
            CacheEntry {
                meta: fresh_meta,
                rope,
            },
        );
        Ok(())
    }
}

fn file_meta(meta: &std::fs::Metadata) -> std::io::Result<FileMeta> {
    let size = meta.len();
    let modified: SystemTime = meta.modified()?;
    let ns: i128 = match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i128) * 1_000_000_000i128 + (d.subsec_nanos() as i128),
        Err(_) => 0, // clock went backwards; treat as 0
    };
    Ok(FileMeta { size, mtime_ns: ns })
}
