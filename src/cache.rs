use crate::protocol::{EngineError, ErrorCode};
use crate::workspace::Workspace;
use lru::LruCache;
use ropey::Rope;
use serde_json::json;
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

pub struct RopeCache {
    entries: LruCache<PathBuf, CacheEntry>,
    max_bytes: usize,
    current_bytes: usize,
}

impl Default for RopeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RopeCache {
    pub fn new() -> Self {
        Self {
            entries: LruCache::unbounded(),
            max_bytes: cache_max_bytes(),
            current_bytes: 0,
        }
    }

    /// Get a "fresh" rope for a file:
    /// - resolve path within repo root
    /// - stat and compare (mtime, size)
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

        // Use peek so we don't promote a stale entry that's about to be replaced.
        let needs_reload = match self.entries.peek(&resolved) {
            None => true,
            Some(entry) => entry.meta != fresh_meta,
        };

        if needs_reload {
            let text = std::fs::read_to_string(&resolved).map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to read file as UTF-8 text")
                    .with_details(json!({ "path": user_path, "io": e.to_string() }))
            })?;

            let rope = Rope::from_str(&text);
            self.insert(
                resolved.clone(),
                CacheEntry {
                    meta: fresh_meta,
                    rope,
                },
            );
        }

        let entry = self.entries.get(&resolved).unwrap();
        Ok((resolved, &entry.rope))
    }

    /// After `apply_patch` writes the file, call this to update the cache
    /// with the new rope and refreshed metadata.
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

        self.insert(
            resolved,
            CacheEntry {
                meta: fresh_meta,
                rope,
            },
        );
        Ok(())
    }

    fn insert(&mut self, path: PathBuf, entry: CacheEntry) {
        let new_bytes = entry.rope.len_bytes();

        // If replacing an existing entry, subtract its contribution first.
        if let Some(old) = self.entries.put(path, entry) {
            self.current_bytes = self.current_bytes.saturating_sub(old.rope.len_bytes());
        }
        self.current_bytes += new_bytes;

        // Evict LRU entries until we're within the limit. Always keep at least
        // one entry (the one we just inserted) even if it alone exceeds the limit,
        // since evicting it would leave us with nothing useful.
        while self.current_bytes > self.max_bytes && self.entries.len() > 1 {
            if let Some((_, evicted)) = self.entries.pop_lru() {
                self.current_bytes = self.current_bytes.saturating_sub(evicted.rope.len_bytes());
            } else {
                break;
            }
        }
    }
}

fn cache_max_bytes() -> usize {
    const DEFAULT_MB: usize = 128;
    std::env::var("AGENT_MCP_CACHE_MB")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .map(|mb| mb.saturating_mul(1024 * 1024))
        .unwrap_or(DEFAULT_MB * 1024 * 1024)
}

fn file_meta(meta: &std::fs::Metadata) -> std::io::Result<FileMeta> {
    let size = meta.len();
    let modified: SystemTime = meta.modified()?;
    let ns: i128 = match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i128) * 1_000_000_000i128 + (d.subsec_nanos() as i128),
        Err(_) => 0,
    };
    Ok(FileMeta { size, mtime_ns: ns })
}
