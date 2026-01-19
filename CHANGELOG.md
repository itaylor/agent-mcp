# Changelog

All notable changes to the Agent MCP Server project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Critical bug in `apply_patch` byte-to-char conversion**: Fixed panic when patching files containing multibyte UTF-8 characters (emoji, CJK characters, etc.). The `byte_to_char_idx` function was incorrectly slicing strings at byte boundaries instead of properly tracking character boundaries, causing crashes on files with non-ASCII content.
  - Added comprehensive tests for UTF-8 character handling in patches
  - All 30 Rust tests now pass including UTF-8 edge cases

### Added

- **Five new MCP operations** for complete file management:
  - `create_directory`: Create directories with automatic parent creation (like `mkdir -p`)
  - `create_file`: Create or overwrite files with UTF-8 content and automatic parent directory creation
  - `read_file_info`: Get file metadata (size, mtime, type, existence) without reading contents
  - `delete_file`: Delete files or directories (recursive for directories)
  - `read_file`: Read entire file contents with hard 100KB limit to prevent memory issues

- **Comprehensive test coverage**:
  - 30 unit tests covering all operations including edge cases
  - Test suite validates UTF-8 handling, path sandboxing, and error conditions
  - Integration test script (`test-mcp.sh`) now tests all 11 operations

- **Node MCP server integration**:
  - All 5 new operations exposed via MCP protocol
  - Proper JSON schema definitions for each tool
  - Consistent error handling and response formatting

### Documentation

- Added TODO comments for deferred improvements:
  - Cache unbounded growth issue (needs LRU eviction in future)
  - `explore_code` exported detection incorrectly marks all class members as exported (affects TypeScript, Rust, and Java)

### Technical Details

#### Bug Fix: UTF-8 Character Boundaries

**Before** (broken):
```rust
for (i, _ch) in chunk.chars().enumerate() {
    if chunk[..i].len() >= offset {  // WRONG: byte slicing with char index
        return char_idx;
    }
}
```

**After** (fixed):
```rust
let mut byte_pos = 0;
for ch in chunk.chars() {
    if byte_pos >= offset {
        return char_idx;
    }
    byte_pos += ch.len_utf8();  // Properly track byte positions
    char_idx += 1;
}
```

#### New Operations Architecture

All new operations follow the established patterns:
- Path sandboxing via `Workspace::resolve_path()`
- Structured error responses with stable error codes
- Comprehensive validation and edge case handling
- Atomic operations where applicable (file writes use temp+rename)

**Operation Sizes**:
- `create_directory`: 126 lines with tests
- `create_file`: 187 lines with tests (includes encoding validation)
- `read_file_info`: 183 lines with tests (includes mtime tracking)
- `delete_file`: 151 lines with tests (handles recursive deletion)
- `read_file`: 233 lines with tests (enforces 100KB hard limit)

### Testing

**Unit Test Results**:
```
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured
```

**Integration Test Results**:
- All 11 operations tested end-to-end via NDJSON protocol
- Validates filesystem state changes
- Tests error conditions (path traversal, missing files, etc.)
- UTF-8 content handling verified

### Known Limitations

1. **Cache Growth**: The Ropey cache grows unbounded. In long-running sessions with hundreds of files, memory usage will increase without limit. LRU eviction needed.

2. **Exported Detection**: The `explore_code` operation incorrectly marks all members of exported classes/structs as `exported: true`. Only top-level exports should be marked as exported.

3. **No Batching**: Each operation requires a separate request/response round-trip. No support for batch operations.

4. **File Creation Only**: New files can only be created with UTF-8 encoding. No support for binary files or other encodings.

### Performance

- All operations maintain O(1) or O(n) complexity
- File operations are atomic where applicable
- Cache invalidation is efficient (mtime/size comparison only)
- Test suite completes in <2 seconds

## [0.1.0] - Initial Release

### Added

- Initial implementation of Agent MCP Server
- Two-process architecture (Node MCP server + Rust code engine)
- NDJSON protocol over stdin/stdout
- Core operations:
  - `list_dir`: Directory traversal with depth control
  - `search_text`: Regex search via ripgrep
  - `read_excerpt`: Line-range file reading
  - `explore_code`: Tree-sitter symbol extraction
  - `apply_patch`: Anchor-based code editing
- Ropey-based cache with automatic invalidation
- Path sandboxing and security controls
- Support for TypeScript, JavaScript, Rust, and Java code analysis