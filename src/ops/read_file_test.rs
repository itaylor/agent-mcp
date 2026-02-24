use super::*;
use crate::workspace::Workspace;
use std::fs;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_read_small_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let args = ReadFileArgs {
        file_path: "test.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.contents, "Hello, World!");
    assert_eq!(res.size, 13);
    assert!(!res.truncated);
}

#[test]
fn test_read_file_with_utf8() {
    let (_temp_dir, ws) = setup_test_workspace();

    let content = "Hello 世界 🌍\nMultiple lines\nWith UTF-8";
    let file_path = ws.repo_root().join("utf8.txt");
    fs::write(&file_path, content).unwrap();

    let args = ReadFileArgs {
        file_path: "utf8.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.contents, content);
}

#[test]
fn test_read_empty_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("empty.txt");
    fs::write(&file_path, "").unwrap();

    let args = ReadFileArgs {
        file_path: "empty.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.contents, "");
    assert_eq!(res.size, 0);
}

#[test]
fn test_read_file_exceeds_max_bytes() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file larger than the limit
    let large_content = "x".repeat(200_000);
    let file_path = ws.repo_root().join("large.txt");
    fs::write(&file_path, &large_content).unwrap();

    let args = ReadFileArgs {
        file_path: "large.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}

#[test]
fn test_read_file_exactly_at_limit() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file exactly at the limit
    let content = "x".repeat(102_400);
    let file_path = ws.repo_root().join("exact.txt");
    fs::write(&file_path, &content).unwrap();

    let args = ReadFileArgs {
        file_path: "exact.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.size, 102_400);
}

#[test]
fn test_read_nonexistent_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = ReadFileArgs {
        file_path: "nonexistent.txt".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().code, ErrorCode::NotFound));
}

#[test]
fn test_read_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();

    let args = ReadFileArgs {
        file_path: "test_dir".to_string(),
        max_bytes: 102_400,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}

#[test]
fn test_read_file_with_custom_limit() {
    let (_temp_dir, ws) = setup_test_workspace();

    let content = "x".repeat(1000);
    let file_path = ws.repo_root().join("test.txt");
    fs::write(&file_path, &content).unwrap();

    let args = ReadFileArgs {
        file_path: "test.txt".to_string(),
        max_bytes: 500, // Custom smaller limit
    };

    let result = run(&ws, args);
    assert!(result.is_err()); // Should fail because file is larger than custom limit
}

#[test]
fn test_hard_limit_enforced() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file that's 150KB
    let content = "x".repeat(150_000);
    let file_path = ws.repo_root().join("large.txt");
    fs::write(&file_path, &content).unwrap();

    let args = ReadFileArgs {
        file_path: "large.txt".to_string(),
        max_bytes: 200_000, // Try to set limit above 100KB
    };

    let result = run(&ws, args);
    // Should fail because file is over 100KB, even though maxBytes is 200KB
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}
