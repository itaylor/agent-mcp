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
fn test_explore_typescript_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let ts_content = r#"
export function greet(name: string): string {
    return `Hello, ${name}`;
}

export class User {
    constructor(public name: string) {}
}

function internal() {
    return "internal";
}
"#;

    let file_path = ws.repo_root().join("test.ts");
    fs::write(&file_path, ts_content).unwrap();

    // Test with exported=true
    let args = ExploreCodeArgs {
        file_path: "test.ts".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.language, "ts");
    assert!(res.symbols.len() >= 2); // greet and User

    // Should have function and class
    assert!(res.symbols.iter().any(|s| s.kind == "function"));
    assert!(res.symbols.iter().any(|s| s.kind == "class"));
}

#[test]
fn test_explore_rust_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let rust_content = r#"
pub fn public_function() -> i32 {
    42
}

fn private_function() {
    println!("private");
}

pub struct MyStruct {
    pub field: String,
}

pub enum MyEnum {
    Variant1,
    Variant2,
}
"#;

    let file_path = ws.repo_root().join("test.rs");
    fs::write(&file_path, rust_content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.rs".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.language, "rust");
    assert!(res.symbols.len() >= 3); // public_function, MyStruct, MyEnum

    // Check for expected kinds
    assert!(res.symbols.iter().any(|s| s.kind == "function"));
    assert!(res.symbols.iter().any(|s| s.kind == "struct"));
    assert!(res.symbols.iter().any(|s| s.kind == "enum"));
}

#[test]
fn test_explore_java_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let java_content = r#"
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }

    private void helper() {
        // helper method
    }
}

public interface MyInterface {
    void doSomething();
}
"#;

    let file_path = ws.repo_root().join("test.java");
    fs::write(&file_path, java_content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.java".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.language, "java");
    assert!(res.symbols.len() >= 2); // HelloWorld class and MyInterface

    assert!(res.symbols.iter().any(|s| s.kind == "class"));
    assert!(res.symbols.iter().any(|s| s.kind == "interface"));
}

#[test]
fn test_explore_javascript_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let js_content = r#"
export function add(a, b) {
    return a + b;
}

export const multiply = (a, b) => a * b;

function internal() {
    return 42;
}
"#;

    let file_path = ws.repo_root().join("test.js");
    fs::write(&file_path, js_content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.js".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.language, "js");
    assert!(res.symbols.len() >= 2); // add and multiply
}

#[test]
fn test_explore_include_non_exported() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let ts_content = r#"
export function exported() {}
function notExported() {}
"#;

    let file_path = ws.repo_root().join("test.ts");
    fs::write(&file_path, ts_content).unwrap();

    // Test with exported=false (include all)
    let args = ExploreCodeArgs {
        file_path: "test.ts".to_string(),
        exported: false,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // Should include both exported and non-exported
    assert!(res.symbols.len() >= 2);
}

#[test]
fn test_explore_max_symbols_truncation() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    // Create file with many functions
    let mut content = String::from("export function f1() {}\n");
    for i in 2..=20 {
        content.push_str(&format!("export function f{i}() {{}}\n"));
    }

    let file_path = ws.repo_root().join("test.ts");
    fs::write(&file_path, content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.ts".to_string(),
        exported: true,
        max_symbols: 5,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.symbols.len(), 5); // Truncated to max_symbols
    assert!(res.notes.is_some());
    assert!(res.notes.unwrap()[0].contains("truncated"));
}

#[test]
fn test_explore_unsupported_file_type() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let file_path = ws.repo_root().join("test.py");
    fs::write(&file_path, "def hello(): pass").unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.py".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::InvalidArgument
    ));
}

#[test]
fn test_explore_nonexistent_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let args = ExploreCodeArgs {
        file_path: "nonexistent.ts".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().code, ErrorCode::NotFound));
}

#[test]
fn test_explore_symbol_locations() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let ts_content = r#"export function first() {
    return 1;
}

export function second() {
    return 2;
}
"#;

    let file_path = ws.repo_root().join("test.ts");
    fs::write(&file_path, ts_content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.ts".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    // All symbols should have valid line locations
    for symbol in &res.symbols {
        assert!(symbol.location.start_line > 0);
        assert!(symbol.location.end_line >= symbol.location.start_line);
    }
}

#[test]
fn test_explore_tsx_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let tsx_content = r#"
export const MyComponent = () => {
    return <div>Hello</div>;
};

export interface Props {
    name: string;
}
"#;

    let file_path = ws.repo_root().join("test.tsx");
    fs::write(&file_path, tsx_content).unwrap();

    let args = ExploreCodeArgs {
        file_path: "test.tsx".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.language, "tsx");
    assert!(res.symbols.len() >= 2); // MyComponent and Props
}

#[test]
fn test_explore_empty_file() {
    let (_temp_dir, ws) = setup_test_workspace();
    let mut cache = RopeCache::new();

    let file_path = ws.repo_root().join("empty.ts");
    fs::write(&file_path, "").unwrap();

    let args = ExploreCodeArgs {
        file_path: "empty.ts".to_string(),
        exported: true,
        max_symbols: 200,
    };

    let result = run(&ws, &mut cache, args);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.symbols.len(), 0);
}
