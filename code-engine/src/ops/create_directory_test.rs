use super::*;
use crate::workspace::Workspace;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_create_single_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateDirectoryArgs {
        dir_path: "test_dir".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().created);

    let dir_path = ws.repo_root().join("test_dir");
    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
}

#[test]
fn test_create_nested_directories() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = CreateDirectoryArgs {
        dir_path: "parent/child/grandchild".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(result.unwrap().created);

    let dir_path = ws.repo_root().join("parent/child/grandchild");
    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
}

#[test]
fn test_create_already_existing_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("existing");
    fs::create_dir(&dir_path).unwrap();

    let args = CreateDirectoryArgs {
        dir_path: "existing".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    assert!(!result.unwrap().created); // Should report not created since it existed
}

#[test]
fn test_create_when_file_exists() {
    let (_temp_dir, ws) = setup_test_workspace();

    let file_path = ws.repo_root().join("existing_file");
    fs::write(&file_path, "content").unwrap();

    let args = CreateDirectoryArgs {
        dir_path: "existing_file".to_string(),
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}
