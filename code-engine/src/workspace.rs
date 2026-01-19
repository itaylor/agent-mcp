use crate::protocol::{EngineError, ErrorCode};
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct Workspace {
    repo_root: PathBuf,
}

impl Workspace {
    pub fn new(repo_root: String) -> anyhow::Result<Self> {
        // Canonicalize to a stable absolute path.
        // If it doesn't exist yet, canonicalize will fail; you can create or error.
        let root = std::fs::canonicalize(&repo_root)?;
        Ok(Self { repo_root: root })
    }

    #[allow(dead_code)]
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Resolve a user-supplied relative path within repo_root.
    /// Reject absolute paths and any attempt to escape via `..`.
    pub fn resolve_path(&self, user_path: &str) -> Result<PathBuf, EngineError> {
        let p = Path::new(user_path);

        if p.is_absolute() {
            return Err(EngineError::new(
                ErrorCode::PathOutsideRoot,
                "Absolute paths are not allowed",
            )
            .with_details(json!({ "path": user_path })));
        }

        // Join and then normalize by canonicalizing the parent when possible.
        // Note: canonicalize requires existence. For non-existent paths (e.g., future files),
        // you can canonicalize the nearest existing ancestor; keep it simple for now.
        let joined = self.repo_root.join(p);

        // Reject obvious traversal segments early
        if user_path.contains("..") {
            // Still allow "foo..bar" — this is simplistic. Better: inspect components.
            // We'll do component-based check:
            for c in p.components() {
                if matches!(c, std::path::Component::ParentDir) {
                    return Err(EngineError::new(
                        ErrorCode::PathOutsideRoot,
                        "Path traversal is not allowed",
                    )
                    .with_details(json!({ "path": user_path })));
                }
            }
        }

        // If exists, canonicalize and ensure under root
        if joined.exists() {
            let can = std::fs::canonicalize(&joined).map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to canonicalize path")
                    .with_details(json!({ "path": user_path, "io": e.to_string() }))
            })?;
            if !can.starts_with(&self.repo_root) {
                return Err(EngineError::new(
                    ErrorCode::PathOutsideRoot,
                    "Resolved path is outside repo root",
                )
                .with_details(json!({ "path": user_path })));
            }
            Ok(can)
        } else {
            // For non-existent targets, ensure the *parent* is inside root if parent exists.
            let parent = joined.parent().unwrap_or(&self.repo_root);
            if parent.exists() {
                let can_parent = std::fs::canonicalize(parent).map_err(|e| {
                    EngineError::new(ErrorCode::IoError, "Failed to canonicalize parent")
                        .with_details(json!({ "path": user_path, "io": e.to_string() }))
                })?;
                if !can_parent.starts_with(&self.repo_root) {
                    return Err(EngineError::new(
                        ErrorCode::PathOutsideRoot,
                        "Resolved parent is outside repo root",
                    )
                    .with_details(json!({ "path": user_path })));
                }
            }
            Ok(joined)
        }
    }
}
