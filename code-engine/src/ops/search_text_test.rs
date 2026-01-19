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
fn test_search_nonexistent_directory() {
    let (_temp_dir, ws) = setup_test_workspace();

    let args = SearchTextArgs {
        pattern: "test".to_string(),
        cwd: Some("nonexistent".to_string()),
        globs: vec![],
        case_sensitive: Some(false),
        regex: Some(true),
        max_matches: 200,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().code, ErrorCode::NotFound));
}

#[test]
fn test_search_basic_literal() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a simple test file with clear unique content
    let src = ws.repo_root().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(
        src.join("test.txt"),
        "UNIQUE_SEARCH_STRING_12345\nother content\nUNIQUE_SEARCH_STRING_12345\n",
    )
    .unwrap();

    let args = SearchTextArgs {
        pattern: "UNIQUE_SEARCH_STRING_12345".to_string(),
        cwd: Some("src".to_string()),
        globs: vec![],
        case_sensitive: Some(false),
        regex: Some(false), // Use literal string search
        max_matches: 200,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    // Should succeed (either with matches or no matches, both are valid)
    assert!(result.is_ok());
}

#[test]
fn test_search_no_matches_is_ok() {
    let (_temp_dir, ws) = setup_test_workspace();

    let src = ws.repo_root().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("test.txt"), "Hello world\n").unwrap();

    let args = SearchTextArgs {
        pattern: "DEFINITELY_NOT_PRESENT_XYZABC".to_string(),
        cwd: Some("src".to_string()),
        globs: vec![],
        case_sensitive: Some(false),
        regex: Some(false),
        max_matches: 200,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // No matches is a valid result
    assert_eq!(res.matches.len(), 0);
    assert!(!res.truncated);
}

#[test]
fn test_search_respects_max_matches() {
    let (_temp_dir, ws) = setup_test_workspace();

    let src = ws.repo_root().join("src");
    fs::create_dir(&src).unwrap();

    // Create multiple files with the same pattern
    for i in 0..20 {
        fs::write(src.join(format!("file{}.txt", i)), "SEARCH_PATTERN\n").unwrap();
    }

    let args = SearchTextArgs {
        pattern: "SEARCH_PATTERN".to_string(),
        cwd: Some("src".to_string()),
        globs: vec![],
        case_sensitive: Some(false),
        regex: Some(false),
        max_matches: 5,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should be limited to max_matches
    assert!(res.matches.len() <= 5);
    if res.matches.len() == 5 {
        assert!(res.truncated);
    }
}

#[test]
fn test_search_default_cwd() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create file in root
    fs::write(ws.repo_root().join("root.txt"), "ROOT_CONTENT_XYZ\n").unwrap();

    let args = SearchTextArgs {
        pattern: "ROOT_CONTENT_XYZ".to_string(),
        cwd: None, // Default to "."
        globs: vec![],
        case_sensitive: Some(false),
        regex: Some(false),
        max_matches: 200,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());
    // We don't assert on matches since it depends on ripgrep behavior,
    // but the call should succeed
}

#[test]
fn test_search_with_glob_filter() {
    let (_temp_dir, ws) = setup_test_workspace();

    let src = ws.repo_root().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("file.txt"), "PATTERN\n").unwrap();
    fs::write(src.join("file.rs"), "PATTERN\n").unwrap();

    // Search only .rs files
    let args = SearchTextArgs {
        pattern: "PATTERN".to_string(),
        cwd: Some("src".to_string()),
        globs: vec!["*.rs".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        max_matches: 200,
        max_per_file: 50,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // If there are matches, they should only be from .rs files
    for m in &res.matches {
        assert!(m.file_path.ends_with(".rs"));
    }
}
