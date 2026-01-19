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
fn test_list_simple_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a simple directory structure
    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();
    fs::write(dir_path.join("file1.txt"), "content1").unwrap();
    fs::write(dir_path.join("file2.txt"), "content2").unwrap();

    let args = ListDirArgs {
        dir_path: "test_dir".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.entries.len(), 2);
    assert!(!res.truncated);
}

#[test]
fn test_list_with_subdirectories() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create nested structure
    let base = ws.repo_root().join("base");
    fs::create_dir(&base).unwrap();
    fs::write(base.join("root.txt"), "root").unwrap();

    let sub1 = base.join("sub1");
    fs::create_dir(&sub1).unwrap();
    fs::write(sub1.join("file1.txt"), "content1").unwrap();

    let sub2 = base.join("sub2");
    fs::create_dir(&sub2).unwrap();
    fs::write(sub2.join("file2.txt"), "content2").unwrap();

    let args = ListDirArgs {
        dir_path: "base".to_string(),
        depth: 2,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert!(res.entries.len() >= 5); // root.txt, sub1, sub1/file1.txt, sub2, sub2/file2.txt
    assert!(!res.truncated);
}

#[test]
fn test_list_depth_limit() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create deep nested structure
    let base = ws.repo_root().join("base");
    fs::create_dir(&base).unwrap();
    let level1 = base.join("level1");
    fs::create_dir(&level1).unwrap();
    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    fs::write(level2.join("deep.txt"), "deep").unwrap();

    // List with depth 1 - should not see level2
    let args = ListDirArgs {
        dir_path: "base".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should see level1 directory but not its contents
    assert!(res.entries.iter().any(|e| e.path == "level1"));
    assert!(!res.entries.iter().any(|e| e.path.contains("level2")));
}

#[test]
fn test_list_hidden_files() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();
    fs::write(dir_path.join("visible.txt"), "visible").unwrap();
    fs::write(dir_path.join(".hidden.txt"), "hidden").unwrap();

    // Without includeHidden
    let args = ListDirArgs {
        dir_path: "test_dir".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(!res.entries.iter().any(|e| e.path.contains(".hidden")));

    // With includeHidden
    let args = ListDirArgs {
        dir_path: "test_dir".to_string(),
        depth: 1,
        include_hidden: true,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.entries.iter().any(|e| e.path.contains(".hidden")));
}

#[test]
fn test_list_max_entries_truncation() {
    let (_temp_dir, ws) = setup_test_workspace();

    let dir_path = ws.repo_root().join("test_dir");
    fs::create_dir(&dir_path).unwrap();

    // Create 10 files
    for i in 0..10 {
        fs::write(dir_path.join(format!("file{}.txt", i)), "content").unwrap();
    }

    // Limit to 5 entries
    let args = ListDirArgs {
        dir_path: "test_dir".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 5,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.entries.len(), 5);
    assert!(res.truncated);
}

#[test]
fn test_list_nonexistent_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = ListDirArgs {
        dir_path: "nonexistent".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().code, ErrorCode::NotFound));
}

#[test]
fn test_list_file_not_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file
    let file_path = ws.repo_root().join("file.txt");
    fs::write(&file_path, "content").unwrap();

    let args = ListDirArgs {
        dir_path: "file.txt".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}

#[test]
fn test_list_entry_types() {
    let (_temp_dir, ws) = setup_test_workspace();

    let base = ws.repo_root().join("base");
    fs::create_dir(&base).unwrap();
    fs::write(base.join("file.txt"), "content").unwrap();
    fs::create_dir(base.join("subdir")).unwrap();

    let args = ListDirArgs {
        dir_path: "base".to_string(),
        depth: 1,
        include_hidden: false,
        max_entries: 100,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();

    let file_entry = res.entries.iter().find(|e| e.path == "file.txt").unwrap();
    assert_eq!(file_entry.entry_type, "file");
    assert!(file_entry.size.is_some());

    let dir_entry = res.entries.iter().find(|e| e.path == "subdir").unwrap();
    assert_eq!(dir_entry.entry_type, "dir");
}
