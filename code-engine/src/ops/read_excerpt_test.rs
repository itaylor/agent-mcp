use super::*;
use crate::cache::RopeCache;
use crate::workspace::Workspace;
use std::fs;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_hard_limit_enforced() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    // Create a file with 150KB of content (1500 lines of 100 bytes each)
    let line = "x".repeat(100);
    let content = (0..1500)
        .map(|_| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let file_path = ws.repo_root().join("large.txt");
    fs::write(&file_path, &content).unwrap();

    let args = ReadExcerptArgs {
        file_path: "large.txt".to_string(),
        start_line: 1,
        end_line: 1500,
        max_bytes: 200_000, // Try to set limit above 100KB
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should be truncated to 100KB even though we requested 200KB
    assert!(res.truncated);
    assert!(res.text.len() <= 102_400);
}

#[test]
fn test_normal_read_under_limit() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let content = "line 1\nline 2\nline 3\n";
    let file_path = ws.repo_root().join("small.txt");
    fs::write(&file_path, content).unwrap();

    let args = ReadExcerptArgs {
        file_path: "small.txt".to_string(),
        start_line: 1,
        end_line: 3,
        max_bytes: 20_000,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert!(!res.truncated);
    assert_eq!(res.text, "line 1\nline 2\nline 3\n");
}
