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
fn test_delete_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("test.txt");
    fs::write(&file_path, "content").unwrap();
    assert!(file_path.exists());

    let args = DeleteFileArgs {
        file_path: "test.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().deleted);
    assert!(!file_path.exists());
}

#[test]
fn test_delete_empty_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();
    assert!(dir_path.exists());

    let args = DeleteFileArgs {
        file_path: "test_dir".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().deleted);
    assert!(!dir_path.exists());
}

#[test]
fn test_delete_directory_with_contents() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();
    fs::write(dir_path.join("file1.txt"), "content1").unwrap();
    fs::write(dir_path.join("file2.txt"), "content2").unwrap();
    fs::create_dir(dir_path.join("subdir")).unwrap();
    fs::write(dir_path.join("subdir/file3.txt"), "content3").unwrap();

    let args = DeleteFileArgs {
        file_path: "test_dir".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().deleted);
    assert!(!dir_path.exists());
}

#[test]
fn test_delete_nonexistent_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = DeleteFileArgs {
        file_path: "nonexistent.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().code, ErrorCode::NotFound));
}

#[test]
fn test_delete_nested_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("dir1/dir2");
    fs::create_dir_all(&dir_path).unwrap();
    let file_path = dir_path.join("test.txt");
    fs::write(&file_path, "content").unwrap();

    let args = DeleteFileArgs {
        file_path: "dir1/dir2/test.txt".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(!file_path.exists());
    // Parent directories should still exist
    assert!(dir_path.exists());
}
