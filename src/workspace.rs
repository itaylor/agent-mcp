use crate::protocol::{EngineError, ErrorCode};
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct Workspace {
    repo_root: PathBuf,
}

impl Workspace {
    pub fn new(repo_root: String) -> anyhow::Result<Self> {
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

        let joined = self.repo_root.join(p);

        if user_path.contains("..") {
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
