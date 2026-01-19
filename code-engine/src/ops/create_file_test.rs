use super::*;
use crate::workspace::Workspace;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_create_simple_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateFileArgs {
        file_path: "test.txt".to_string(),
        contents: "Hello, World!".to_string(),
        encoding: "utf8".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.created);
    assert_eq!(res.bytes_written, 13);

    let file_path = ws.repo_root().join("test.txt");
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "Hello, World!");
}

#[test]
fn test_create_file_with_nested_path() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateFileArgs {
        file_path: "dir1/dir2/test.txt".to_string(),
        contents: "Nested file".to_string(),
        encoding: "utf8".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().created);

    let file_path = ws.repo_root().join("dir1/dir2/test.txt");
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "Nested file");
}

#[test]
fn test_overwrite_existing_file() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("existing.txt");
    fs::write(&file_path, "Original content").unwrap();

    let args = CreateFileArgs {
        file_path: "existing.txt".to_string(),
        contents: "New content".to_string(),
        encoding: "utf8".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(!res.created); // Should report not created since it existed
    assert_eq!(res.bytes_written, 11);

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "New content");
}

#[test]
fn test_create_file_utf8_content() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateFileArgs {
        file_path: "utf8_test.txt".to_string(),
        contents: "Hello 世界 🌍".to_string(),
        encoding: "utf8".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let file_path = ws.repo_root().join("utf8_test.txt");
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "Hello 世界 🌍");
}

#[test]
fn test_create_file_where_directory_exists() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("existing_dir");
    fs::create_dir(&dir_path).unwrap();

    let args = CreateFileArgs {
        file_path: "existing_dir".to_string(),
        contents: "content".to_string(),
        encoding: "utf8".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}

#[test]
fn test_invalid_encoding() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateFileArgs {
        file_path: "test.txt".to_string(),
        contents: "content".to_string(),
        encoding: "latin1".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}
