use super::*;
use crate::cache::RopeCache;
use crate::protocol::{ApplyPatchArgs, PatchAnchor, PatchEdit};
use crate::workspace::Workspace;
use std::fs;
use tempfile::TempDir;

fn setup_test_workspace() -> (TempDir, Workspace) {
    let temp_dir = TempDir::new().unwrap();
    let ws = Workspace::new(temp_dir.path().to_string_lossy().to_string()).unwrap();
    (temp_dir, ws)
}

#[test]
fn test_apply_patch_with_utf8_characters() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    // Create a test file with UTF-8 characters (emoji and accents)
    let test_content = "// Start of file\n\
                   fn hello() {\n\
                       println!(\"Hello 世界 🌍\");\n\
                   }\n\
                   // End of file\n";

    let test_file = ws.repo_root().join("test.rs");
    fs::write(&test_file, test_content).unwrap();

    // Try to insert text after the UTF-8 content
    let args = ApplyPatchArgs {
        file_path: "test.rs".to_string(),
        mode: Some("strict".to_string()),
        dry_run: false,
        edits: vec![PatchEdit::Insert {
            where_: "after".to_string(),
            anchor: PatchAnchor {
                needle: "println!(\"Hello 世界 🌍\");".to_string(),
                require_unique: Some(true),
            },
            text: "\n    println!(\"Goodbye!\");".to_string(),
        }],
    };

    // This should work but will fail due to byte/char conversion bug
    let result = run(&ws, &mut cache, args);
    assert!(
        result.is_ok(),
        "Failed to apply patch with UTF-8: {:?}",
        result
    );

    // Verify the result
    let modified_content = fs::read_to_string(&test_file).unwrap();
    assert!(
        modified_content.contains("println!(\"Goodbye!\")"),
        "Patch was not applied correctly"
    );
}

#[test]
fn test_apply_patch_simple_insert() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    // Create a simple test file
    let test_content = "line 1\nline 2\nline 3\n";
    let test_file = ws.repo_root().join("simple.txt");
    fs::write(&test_file, test_content).unwrap();

    let args = ApplyPatchArgs {
        file_path: "simple.txt".to_string(),
        mode: Some("strict".to_string()),
        dry_run: false,
        edits: vec![PatchEdit::Insert {
            where_: "after".to_string(),
            anchor: PatchAnchor {
                needle: "line 2".to_string(),
                require_unique: Some(true),
            },
            text: "\ninserted line".to_string(),
        }],
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok(), "Failed to apply simple patch: {:?}", result);

    let modified_content = fs::read_to_string(&test_file).unwrap();
    assert!(modified_content.contains("inserted line"));
}
