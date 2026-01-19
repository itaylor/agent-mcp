use crate::cache::RopeCache;
use crate::protocol::{
    EngineError, ErrorCode, ExploreCodeArgs, ExploreCodeResult, ExploredSymbol, SymbolLocation,
};
use crate::workspace::Workspace;
use serde_json::json;
use tree_sitter::{Language, Parser, Tree};

pub fn run(
    ws: &Workspace,
    cache: &mut RopeCache,
    args: ExploreCodeArgs,
) -> Result<ExploreCodeResult, EngineError> {
    let (_resolved, rope) = cache.get_fresh_rope(ws, &args.file_path)?;

    // Detect language from file extension
    let language_info = detect_language(&args.file_path)?;

    // Parse with tree-sitter
    let mut parser = Parser::new();
    parser.set_language(language_info.language).map_err(|e| {
        EngineError::new(ErrorCode::ParseFailed, "Failed to set language")
            .with_details(json!({ "error": e.to_string() }))
    })?;

    let source = rope.to_string();
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| EngineError::new(ErrorCode::ParseFailed, "Failed to parse file"))?;

    // Extract symbols based on language
    let mut symbols = match language_info.name {
        "ts" | "tsx" | "js" => extract_ts_symbols(&tree, &source, args.exported),
        "rust" => extract_rust_symbols(&tree, &source, args.exported),
        "java" => extract_java_symbols(&tree, &source, args.exported),
        _ => Vec::new(),
    };

    // Limit to max_symbols
    let truncated = symbols.len() > args.max_symbols;
    symbols.truncate(args.max_symbols);

    let notes = if truncated {
        Some(vec![format!(
            "Symbol list truncated to {} symbols",
            args.max_symbols
        )])
    } else {
        None
    };

    Ok(ExploreCodeResult {
        file_path: args.file_path,
        language: language_info.name.to_string(),
        symbols,
        notes,
    })
}

struct LanguageInfo {
    name: &'static str,
    language: Language,
}

fn detect_language(file_path: &str) -> Result<LanguageInfo, EngineError> {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "ts" => Ok(LanguageInfo {
            name: "ts",
            language: tree_sitter_typescript::language_typescript(),
        }),
        "tsx" => Ok(LanguageInfo {
            name: "tsx",
            language: tree_sitter_typescript::language_tsx(),
        }),
        "js" | "jsx" => Ok(LanguageInfo {
            name: "js",
            language: tree_sitter_typescript::language_typescript(),
        }),
        "rs" => Ok(LanguageInfo {
            name: "rust",
            language: tree_sitter_rust::language(),
        }),
        "java" => Ok(LanguageInfo {
            name: "java",
            language: tree_sitter_java::language(),
        }),
        _ => Err(
            EngineError::new(ErrorCode::InvalidArgument, "Unsupported file type")
                .with_details(json!({ "extension": ext })),
        ),
    }
}

fn extract_ts_symbols(tree: &Tree, source: &str, filter_exported: bool) -> Vec<ExploredSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();

    fn visit_node(
        node: tree_sitter::Node,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        filter_exported: bool,
        symbols: &mut Vec<ExploredSymbol>,
    ) {
        let kind = node.kind();

        match kind {
            "function_declaration" | "method_definition" | "arrow_function" => {
                if let Some(symbol) = extract_ts_function(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "class_declaration" => {
                if let Some(symbol) = extract_ts_class(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "interface_declaration" => {
                if let Some(symbol) = extract_ts_interface(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "type_alias_declaration" => {
                if let Some(symbol) = extract_ts_type_alias(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "enum_declaration" => {
                if let Some(symbol) = extract_ts_enum(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_ts_variables(node, source, filter_exported, symbols);
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                visit_node(cursor.node(), cursor, source, filter_exported, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    if cursor.goto_first_child() {
        loop {
            visit_node(
                cursor.node(),
                &mut cursor,
                source,
                filter_exported,
                &mut symbols,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    symbols
}

// TODO: This exported detection is incorrect. It marks ALL methods/properties inside
// an exported class as exported=true, when only the class itself should be marked as
// exported. The same issue exists in is_exported_rust and is_exported_java.
// We need to differentiate between:
// - Top-level exports (export function foo, export class Bar)
// - Members of exported containers (methods inside exported classes should be exported=false)
fn is_exported_ts(node: tree_sitter::Node, _source: &str) -> bool {
    // Check if node or parent has export keyword
    let mut current = node;
    loop {
        for i in 0..current.child_count() {
            if let Some(child) = current.child(i) {
                if child.kind() == "export" {
                    return true;
                }
            }
        }
        if let Some(parent) = current.parent() {
            if parent.kind() == "export_statement" {
                return true;
            }
            current = parent;
        } else {
            break;
        }
    }
    false
}

fn extract_ts_function(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<anonymous>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "function".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_ts_class(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<anonymous>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "class".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_ts_interface(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "interface".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_ts_type_alias(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "type".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_ts_enum(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "enum".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_ts_variables(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
    symbols: &mut Vec<ExploredSymbol>,
) {
    let exported = is_exported_ts(node, source);
    if filter_exported && !exported {
        return;
    }

    // Look for variable_declarator children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        symbols.push(ExploredSymbol {
                            name: name.to_string(),
                            kind: "const".to_string(),
                            exported,
                            signature: None,
                            location: node_to_location(child),
                        });
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_rust_symbols(tree: &Tree, source: &str, filter_exported: bool) -> Vec<ExploredSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();

    fn visit_node(
        node: tree_sitter::Node,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        filter_exported: bool,
        symbols: &mut Vec<ExploredSymbol>,
    ) {
        let kind = node.kind();

        match kind {
            "function_item" => {
                if let Some(symbol) = extract_rust_function(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "struct_item" => {
                if let Some(symbol) = extract_rust_struct(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "enum_item" => {
                if let Some(symbol) = extract_rust_enum(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "trait_item" => {
                if let Some(symbol) = extract_rust_trait(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "mod_item" => {
                if let Some(symbol) = extract_rust_module(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "const_item" | "static_item" => {
                if let Some(symbol) = extract_rust_const(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                visit_node(cursor.node(), cursor, source, filter_exported, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    if cursor.goto_first_child() {
        loop {
            visit_node(
                cursor.node(),
                &mut cursor,
                source,
                filter_exported,
                &mut symbols,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    symbols
}

fn is_exported_rust(node: tree_sitter::Node, source: &str) -> bool {
    // Check for pub visibility
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "visibility_modifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return text.starts_with("pub") && text == "pub";
                }
            }
        }
    }
    false
}

fn extract_rust_function(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "function".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_rust_struct(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "struct".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_rust_enum(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "enum".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_rust_trait(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "trait".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_rust_module(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "module".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_rust_const(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_rust(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "const".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_java_symbols(tree: &Tree, source: &str, filter_exported: bool) -> Vec<ExploredSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();

    fn visit_node(
        node: tree_sitter::Node,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        filter_exported: bool,
        symbols: &mut Vec<ExploredSymbol>,
    ) {
        let kind = node.kind();

        match kind {
            "class_declaration" => {
                if let Some(symbol) = extract_java_class(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "interface_declaration" => {
                if let Some(symbol) = extract_java_interface(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "enum_declaration" => {
                if let Some(symbol) = extract_java_enum(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "method_declaration" => {
                if let Some(symbol) = extract_java_method(node, source, filter_exported) {
                    symbols.push(symbol);
                }
            }
            "field_declaration" => {
                extract_java_fields(node, source, filter_exported, symbols);
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                visit_node(cursor.node(), cursor, source, filter_exported, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    if cursor.goto_first_child() {
        loop {
            visit_node(
                cursor.node(),
                &mut cursor,
                source,
                filter_exported,
                &mut symbols,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    symbols
}

fn is_exported_java(node: tree_sitter::Node, source: &str) -> bool {
    // Check for public modifier
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return text.contains("public");
                }
            }
        }
    }
    false
}

fn extract_java_class(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_java(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "class".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_java_interface(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_java(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "interface".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_java_enum(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_java(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "enum".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_java_method(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
) -> Option<ExploredSymbol> {
    let exported = is_exported_java(node, source);
    if filter_exported && !exported {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unnamed>")
        .to_string();

    Some(ExploredSymbol {
        name,
        kind: "method".to_string(),
        exported,
        signature: None,
        location: node_to_location(node),
    })
}

fn extract_java_fields(
    node: tree_sitter::Node,
    source: &str,
    filter_exported: bool,
    symbols: &mut Vec<ExploredSymbol>,
) {
    let exported = is_exported_java(node, source);
    if filter_exported && !exported {
        return;
    }

    // Look for variable_declarator children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        symbols.push(ExploredSymbol {
                            name: name.to_string(),
                            kind: "field".to_string(),
                            exported,
                            signature: None,
                            location: node_to_location(child),
                        });
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn node_to_location(node: tree_sitter::Node) -> SymbolLocation {
    SymbolLocation {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
    }
}

#[cfg(test)]
#[path = "explore_code_test.rs"]
mod explore_code_test;
