# Agent MCP Server

A high-performance MCP (Model Context Protocol) server for code operations, designed for LLM agentic workflows. This server provides file system navigation, text search, code analysis, and intelligent patch application capabilities.

## Architecture

This project uses a **two-process architecture** for optimal performance:

1. **Node.js MCP Server** (thin shim)
   - Implements the official MCP SDK
   - Exposes tools via MCP protocol
   - Handles request validation and routing

2. **Rust Code Engine** (heavy lifting)
   - Single long-lived subprocess
   - Owns all filesystem operations
   - Maintains intelligent Ropey-based cache with mtime/size invalidation
   - Implements search (via ripgrep), parsing (via tree-sitter), and patch application

**Communication**: NDJSON over stdin/stdout between Node and Rust processes.

## Features

### Tools Provided

1. **`list_dir`** - Directory tree listing with depth control and hidden file filtering
2. **`search_text`** - Fast text search using ripgrep with regex support
3. **`read_excerpt`** - Read specific line ranges from files (hard limit: 100KB per request)
4. **`read_file`** - Read entire file contents (hard limit: 100KB, cannot be exceeded)
5. **`explore_code`** - Extract code symbols (functions, classes, interfaces, etc.) using tree-sitter
6. **`apply_patch`** - Anchor-based code editing with strict drift detection
7. **`create_directory`** - Create directories with automatic parent directory creation
8. **`create_file`** - Create or overwrite files with UTF-8 content
9. **`read_file_info`** - Get file metadata (size, mtime, type, existence) without reading contents
10. **`delete_file`** - Delete files or directories recursively

### Key Capabilities

- **Zero explicit versioning**: Cache automatically detects external file changes via (mtime, size) comparison
- **Anchor-based patching**: Edit files by specifying text anchors rather than line numbers
- **Drift detection**: Fails fast when code has changed unexpectedly
- **Multi-language support**: TypeScript/JavaScript, Rust, and Java code parsing
- **Atomic writes**: File changes use temp-file + rename for safety
- **Path sandboxing**: All operations confined to specified repo root

## Prerequisites

- **Rust**: 1.70+ with Cargo
- **Node.js**: 18.0+
- **ripgrep**: Must be installed and available in PATH
- **Git**: For cloning the repository

### Install ripgrep

```bash
# macOS
brew install ripgrep

# Ubuntu/Debian
apt install ripgrep

# Arch Linux
pacman -S ripgrep

# Windows
choco install ripgrep
```

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd agent-mcp
```

### 2. Build Rust Engine

```bash
cd code-engine
cargo build --release
cd ..
```

The compiled binary will be at `code-engine/target/release/code-engine`.

### 3. Setup Node Server

```bash
cd mcp-server
npm install
npm run build
cd ..
```

## Usage

### Running the MCP Server

The server reads from stdin and writes to stdout (MCP protocol via stdio):

```bash
# Set environment variables (optional)
export REPO_ROOT=/path/to/your/repo
export CODE_ENGINE_PATH=/path/to/code-engine/binary

# Start the server
cd mcp-server
npm start
```

### Configuration via Environment Variables

- `REPO_ROOT`: Root directory to operate on (default: current directory)
- `CODE_ENGINE_PATH`: Path to Rust engine binary (default: `../code-engine/target/release/code-engine`)

### Using with MCP Clients

This server implements the standard MCP protocol and can be used with any MCP-compatible client.

#### Example MCP Configuration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "agent-code": {
      "command": "node",
      "args": ["/path/to/agent-mcp/mcp-server/dist/index.js"],
      "env": {
        "REPO_ROOT": "/path/to/target/repository"
      }
    }
  }
}
```

## Tool Usage Examples

### list_dir

```json
{
  "dirPath": "src",
  "depth": 3,
  "includeHidden": false,
  "maxEntries": 1000
}
```

### search_text

```json
{
  "pattern": "function\\s+\\w+",
  "cwd": "src",
  "globs": ["*.ts", "*.js"],
  "caseSensitive": false,
  "regex": true,
  "maxMatches": 100
}
```

### read_excerpt

Read specific line ranges from files. The `maxBytes` parameter has a hard limit of 100KB (102,400 bytes) that cannot be exceeded - any value above this will be capped to 100KB. Default is 20KB.

```json
{
  "filePath": "src/index.ts",
  "startLine": 10,
  "endLine": 30,
  "maxBytes": 10000
}
```

### explore_code

```json
{
  "filePath": "src/server.ts",
  "exported": true,
  "maxSymbols": 200
}
```

### apply_patch

```json
{
  "filePath": "src/example.ts",
  "mode": "strict",
  "dryRun": false,
  "edits": [
    {
      "kind": "insert",
      "where": "after",
      "anchor": {
        "needle": "import express from 'express';",
        "requireUnique": true
      },
      "text": "\nimport cors from 'cors';"
    }
  ]
}
```

### create_directory

```json
{
  "dirPath": "src/components/new-feature"
}
```

### create_file

```json
{
  "filePath": "src/config/settings.json",
  "contents": "{\n  \"apiUrl\": \"https://api.example.com\"\n}",
  "encoding": "utf8"
}
```

### read_file_info

```json
{
  "filePath": "src/index.ts"
}
```

Returns:
```json
{
  "filePath": "src/index.ts",
  "exists": true,
  "entryType": "file",
  "size": 2048,
  "mtimeNs": 1768798629128117000
}
```

### delete_file

```json
{
  "filePath": "src/temp/old-file.ts"
}
```

### read_file

Read entire file contents. The `maxBytes` parameter has a hard limit of 100KB (102,400 bytes) that cannot be exceeded - any value above this will be capped to 100KB. For larger files, use `read_excerpt` instead.

```json
{
  "filePath": "src/config.json",
  "maxBytes": 102400
}
```

## Development

### Rust Development

```bash
cd code-engine

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- /path/to/repo

# Lint
cargo clippy
```

### Node Development

```bash
cd mcp-server

# Rebuild TypeScript
npm run build

# Clean build artifacts
npm run clean
```

### Testing the Pipeline

You can test the Rust engine directly via NDJSON:

```bash
cd code-engine
cargo build --release

# Test with echo
echo '{"id":"test1","op":"list_dir","args":{"dirPath":"."}}' | ./target/release/code-engine .
```

## Cache Behavior

The Rust engine maintains a cache of parsed files (as Ropey objects) with metadata:

- **Cache key**: Canonical absolute file path
- **Metadata**: File size and modification time (nanosecond precision)
- **Invalidation**: Automatic on next access if (size, mtime) differs
- **Impact**: External edits (from IDE, git, etc.) are detected immediately

This means the agent always sees the latest file content without explicit cache clearing.

## Error Handling

The engine returns structured errors with stable error codes:

- `INVALID_ARGUMENT` - Bad input parameters
- `NOT_FOUND` - File/directory doesn't exist
- `PATH_OUTSIDE_ROOT` - Path traversal attempt blocked
- `DRIFT_DETECTED` - Anchor not found or file changed unexpectedly
- `SEARCH_FAILED` - ripgrep execution failed
- `PARSE_FAILED` - tree-sitter parsing failed
- `IO_ERROR` - Filesystem operation failed
- `INTERNAL` - Unexpected error

## Supported Languages

### Code Exploration (tree-sitter)

- TypeScript (`.ts`, `.tsx`)
- JavaScript (`.js`, `.jsx`)
- Rust (`.rs`)
- Java (`.java`)

Additional languages can be added by:
1. Adding tree-sitter grammar dependency to `Cargo.toml`
2. Extending detection logic in `explore_code.rs`
3. Implementing symbol extraction for the grammar

## Performance Notes

- **ripgrep** provides extremely fast text search across large codebases
- **Ropey** enables efficient line-based operations on large files
- **Cache** reduces redundant file I/O and parsing
- **NDJSON streaming** provides low-latency request/response
- **Atomic writes** ensure file integrity even if process crashes

## License

MIT

## Contributing

Contributions welcome! Please ensure:

1. Rust code passes `cargo clippy` and `cargo test`
2. TypeScript compiles without errors
3. All tools maintain backward-compatible JSON schemas
4. Error codes remain stable (part of public API)