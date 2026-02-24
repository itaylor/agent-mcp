# Agent MCP Server

A high-performance MCP (Model Context Protocol) server for code operations, designed for LLM agentic workflows. Provides file system navigation, text search, code analysis, intelligent patch application, and isolated Docker shell execution — all in a single Rust binary.

## Architecture

A single Rust binary that speaks the MCP stdio protocol directly via [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk).

- **MCP transport**: stdio (JSON-RPC over stdin/stdout)
- **File cache**: Ropey-based rope cache with (mtime, size) invalidation
- **Search**: ripgrep subprocess
- **Code parsing**: tree-sitter (TypeScript/JavaScript, Rust, Java)
- **Docker isolation**: persistent container managed internally

## Tools

| Tool | Description |
|------|-------------|
| `list_dir` | Directory tree listing with depth control and hidden file filtering |
| `search_text` | Fast text search via ripgrep with regex support |
| `read_excerpt` | Read specific line ranges from files (hard limit: 100KB) |
| `read_file` | Read entire file contents (hard limit: 100KB) |
| `explore_code` | Extract code symbols using tree-sitter |
| `apply_patch` | Anchor-based code editing with drift detection |
| `create_directory` | Create directories with automatic parent creation |
| `create_file` | Create or overwrite files with UTF-8 content |
| `read_file_info` | Get file metadata without reading contents |
| `delete_file` | Delete files or directories recursively |
| `docker_shell` | Execute commands in a persistent isolated Docker container |

## Prerequisites

- **Rust**: 1.70+ with Cargo
- **ripgrep**: Must be in `PATH`
- **Docker**: Required for `docker_shell` tool only

```bash
# macOS
brew install ripgrep

# Ubuntu/Debian
apt install ripgrep
```

## Build

```bash
cargo build --release
```

The binary will be at `target/release/agent-mcp`.

### Cross-compilation

The `.cargo/config.toml` sets up linkers for common cross targets. Install the appropriate toolchain first:

```bash
# Linux x86_64 musl (static)
rustup target add x86_64-unknown-linux-musl
sudo apt install musl-tools
cargo build --release --target x86_64-unknown-linux-musl

# Linux aarch64 GNU
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# Linux aarch64 musl — use cross
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
```

## Usage

```bash
# Point at a repo root (default: current directory)
REPO_ROOT=/path/to/repo ./agent-mcp

# Or pass it as an argument
./agent-mcp /path/to/repo
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REPO_ROOT` | `.` | Root directory for all file operations |
| `AGENT_MCP_DOCKER_IMAGE` | `ubuntu:24.04` | Docker image for `docker_shell` |

### MCP Client Configuration

```json
{
  "mcpServers": {
    "agent-mcp": {
      "command": "/path/to/agent-mcp",
      "args": ["/path/to/target/repository"],
      "env": {
        "REPO_ROOT": "/path/to/target/repository"
      }
    }
  }
}
```

## Development

```bash
# Run all tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

The `docker_shell` tests detect Docker availability at runtime and self-skip when it's absent, so `cargo test` works without Docker.

## Releases

Pre-built binaries for the following targets are attached to each [GitHub Release](../../releases):

| Target | Description |
|--------|-------------|
| `x86_64-unknown-linux-gnu` | Linux x86_64 (glibc) |
| `x86_64-unknown-linux-musl` | Linux x86_64 (static musl) |
| `aarch64-unknown-linux-gnu` | Linux ARM64 (glibc) |
| `aarch64-unknown-linux-musl` | Linux ARM64 (static musl) |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-apple-darwin` | macOS Intel |
| `x86_64-pc-windows-msvc` | Windows x86_64 |
| `aarch64-pc-windows-msvc` | Windows ARM64 |

## Error Codes

| Code | Meaning |
|------|---------|
| `INVALID_ARGUMENT` | Bad input parameters |
| `NOT_FOUND` | File or directory not found |
| `PATH_OUTSIDE_ROOT` | Path traversal attempt blocked |
| `DRIFT_DETECTED` | Anchor not found or file changed unexpectedly |
| `SEARCH_FAILED` | ripgrep execution failed |
| `PARSE_FAILED` | tree-sitter parsing failed |
| `IO_ERROR` | Filesystem operation failed |
| `INTERNAL` | Unexpected error |

## License

[MIT](LICENSE.md)