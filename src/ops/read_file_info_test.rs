use super::*;
use crate::workspace::Workspace;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_read_file_info_for_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let args = ReadFileInfoArgs {
        file_path: "test.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert!(info.exists);
    assert_eq!(info.entry_type, "file");
    assert_eq!(info.size, Some(13));
    assert!(info.mtime_ns.is_some());
}

#[test]
fn test_read_file_info_for_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();

    let args = ReadFileInfoArgs {
        file_path: "test_dir".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert!(info.exists);
    assert_eq!(info.entry_type, "dir");
    assert!(info.size.is_none()); // Directories don't report size
    assert!(info.mtime_ns.is_some());
}

#[test]
fn test_read_file_info_nonexistent() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = ReadFileInfoArgs {
        file_path: "nonexistent.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert!(!info.exists);
    assert_eq!(info.entry_type, "unknown");
    assert!(info.size.is_none());
    assert!(info.mtime_ns.is_none());
}

#[test]
fn test_read_file_info_mtime_changes() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("test.txt");
    fs::write(&file_path, "Initial").unwrap();

    let args1 = ReadFileInfoArgs {
        file_path: "test.txt".to_string(),
    };

    let result1 = run(&ws, args1);
    let mtime1 = result1.unwrap().mtime_ns;

    // Wait a bit and modify the file
    thread::sleep(Duration::from_millis(10));
    fs::write(&file_path, "Modified").unwrap();

    let args2 = ReadFileInfoArgs {
        file_path: "test.txt".to_string(),
    };

    let result2 = run(&ws, args2);
    let mtime2 = result2.unwrap().mtime_ns;

    // mtime should have changed
    assert_ne!(mtime1, mtime2);
}

#[test]
fn test_read_file_info_empty_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("empty.txt");
    fs::write(&file_path, "").unwrap();

    let args = ReadFileInfoArgs {
        file_path: "empty.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert!(info.exists);
    assert_eq!(info.entry_type, "file");
    assert_eq!(info.size, Some(0));
}
