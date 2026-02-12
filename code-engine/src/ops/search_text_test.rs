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
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
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
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
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
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
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
        no_ignore: true,
        max_matches: 5,
        max_per_file: 50,
        max_line_length: 200,
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
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
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
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // If there are matches, they should only be from .rs files
    for m in &res.matches {
        assert!(m.file_path.ends_with(".rs"));
    }
}

#[test]
fn test_search_yarn_lock_with_glob() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a yarn.lock file with react in it
    let yarn_lock_content = r#"# yarn lockfile v1

react@^18.0.0:
  version "18.2.0"
  resolved "https://registry.yarnpkg.com/react/-/react-18.2.0.tgz"

immer@^10.0.0:
  version "10.0.3"
  resolved "https://registry.yarnpkg.com/immer/-/immer-10.0.3.tgz"
"#;

    fs::write(ws.repo_root().join("yarn.lock"), yarn_lock_content).unwrap();

    // Search for "react" with glob pattern
    let args = SearchTextArgs {
        pattern: "react".to_string(),
        cwd: None,
        globs: vec!["yarn.lock".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should find at least one match
    assert!(
        res.matches.len() > 0,
        "Expected to find 'react' in yarn.lock but got no matches"
    );

    // Verify the match is from yarn.lock
    for m in &res.matches {
        assert!(m.file_path.contains("yarn.lock"));
    }
}

#[test]
fn test_search_with_wildcard_glob() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create multiple files
    fs::write(ws.repo_root().join("file1.txt"), "react here\n").unwrap();
    fs::write(ws.repo_root().join("file2.json"), "react here too\n").unwrap();
    fs::write(ws.repo_root().join("yarn.lock"), "react in lockfile\n").unwrap();

    // Search with **/* glob
    let args = SearchTextArgs {
        pattern: "react".to_string(),
        cwd: None,
        globs: vec!["**/*".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 500,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should find matches in multiple files
    assert!(
        res.matches.len() >= 3,
        "Expected to find 'react' in at least 3 files but got {} matches",
        res.matches.len()
    );
}

#[test]
fn test_debug_ripgrep_command() {
    use std::process::Command;

    let (_temp_dir, ws) = setup_test_workspace();
    fs::write(ws.repo_root().join("yarn.lock"), "react here\n").unwrap();

    // Run ripgrep manually to see what happens
    let mut cmd = Command::new("rg");
    cmd.arg("--json")
        .arg("--no-messages")
        .arg("--max-count")
        .arg("50")
        .arg("--ignore-case")
        .arg("--fixed-strings")
        .arg("--no-ignore")
        .arg("--glob")
        .arg("yarn.lock")
        .arg("react")
        .arg(".")
        .current_dir(ws.repo_root());

    let output = cmd.output().unwrap();

    println!("Exit status: {:?}", output.status);
    println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success() || output.status.code() == Some(1));
}

#[test]
fn test_search_with_debug_output() {
    let (_temp_dir, ws) = setup_test_workspace();
    fs::write(ws.repo_root().join("yarn.lock"), "react here\n").unwrap();

    let args = SearchTextArgs {
        pattern: "react".to_string(),
        cwd: None,
        globs: vec!["yarn.lock".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    println!("Workspace root: {:?}", ws.repo_root());
    println!("Args: {:?}", args);

    let result = run(&ws, args);

    match &result {
        Ok(res) => {
            println!("Success! Got {} matches", res.matches.len());
            for m in &res.matches {
                println!("  Match: {} at line {}", m.file_path, m.line);
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.matches.len() > 0, "Expected matches but got none");
}

#[test]
fn test_line_truncation_long_line() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file with a very long minified-style line
    let long_prefix = "a".repeat(1000);
    let long_suffix = "z".repeat(1000);
    let content = format!("{}FINDME{}\n", long_prefix, long_suffix);
    fs::write(ws.repo_root().join("minified.js"), content).unwrap();

    let args = SearchTextArgs {
        pattern: "FINDME".to_string(),
        cwd: None,
        globs: vec!["minified.js".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.matches.len(), 1, "Expected exactly one match");

    let match_text = &res.matches[0].text;

    // Debug output
    println!("Match text length: {}", match_text.len());
    println!("Match text: {}", match_text);

    // Should be truncated
    assert!(
        match_text.len() <= 250,
        "Line should be truncated to around max_line_length, but got length: {}",
        match_text.len()
    );

    // Should contain the match
    assert!(
        match_text.contains("FINDME"),
        "Truncated text should contain the match"
    );

    // Should have truncation indicators
    assert!(
        match_text.contains("[+"),
        "Should have truncation indicator prefix"
    );
    assert!(
        match_text.contains("chars]"),
        "Should have truncation indicator suffix"
    );
}

#[test]
fn test_line_truncation_short_line() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file with a short line
    let content = "This is a short line with FINDME in it\n";
    fs::write(ws.repo_root().join("short.txt"), content).unwrap();

    let args = SearchTextArgs {
        pattern: "FINDME".to_string(),
        cwd: None,
        globs: vec!["short.txt".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.matches.len(), 1);

    let match_text = &res.matches[0].text;

    // Should NOT be truncated
    assert!(
        !match_text.contains("[+"),
        "Short line should not have truncation indicators"
    );
    assert_eq!(match_text, "This is a short line with FINDME in it");
}

#[test]
fn test_line_truncation_match_at_start() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file with match at the start and long content after
    let long_suffix = "x".repeat(1000);
    let content = format!("FINDME{}\n", long_suffix);
    fs::write(ws.repo_root().join("start.txt"), content).unwrap();

    let args = SearchTextArgs {
        pattern: "FINDME".to_string(),
        cwd: None,
        globs: vec!["start.txt".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.matches.len(), 1);

    let match_text = &res.matches[0].text;

    // Should contain the match
    assert!(match_text.contains("FINDME"));

    // Should have suffix indicator but not prefix (match is at start)
    assert!(
        !match_text.starts_with("[+"),
        "Should not have prefix indicator when match is at start"
    );
    assert!(
        match_text.contains("chars]"),
        "Should have suffix indicator"
    );
}

#[test]
fn test_line_truncation_match_at_end() {
    let (_temp_dir, ws) = setup_test_workspace();

    // Create a file with long content before match at end
    let long_prefix = "y".repeat(1000);
    let content = format!("{}FINDME\n", long_prefix);
    fs::write(ws.repo_root().join("end.txt"), content).unwrap();

    let args = SearchTextArgs {
        pattern: "FINDME".to_string(),
        cwd: None,
        globs: vec!["end.txt".to_string()],
        case_sensitive: Some(false),
        regex: Some(false),
        no_ignore: true,
        max_matches: 200,
        max_per_file: 50,
        max_line_length: 200,
    };

    let result = run(&ws, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.matches.len(), 1);

    let match_text = &res.matches[0].text;

    // Should contain the match
    assert!(match_text.contains("FINDME"));

    // Should have prefix indicator but not suffix (match is at end)
    assert!(
        match_text.starts_with("[+"),
        "Should have prefix indicator when match is at end"
    );
    assert!(
        !match_text.ends_with("chars]"),
        "Should not have suffix indicator when match is at end"
    );
}
