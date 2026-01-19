# Quick Reference Guide

## Installation & Setup

```bash
# 1. Build Rust engine
cd code-engine
cargo build --release

# 2. Setup Node server
cd ../mcp-server
npm install
npm run build

# 3. Test everything
cd ..
./test-mcp.sh
```

## Running the Server

```bash
cd mcp-server
REPO_ROOT=/path/to/repo npm start
```

Or configure in MCP client:
```json
{
  "command": "node",
  "args": ["/path/to/mcp-server/dist/index.js"],
  "env": {"REPO_ROOT": "/path/to/repo"}
}
```

## Tool Reference

### list_dir

List directory contents with depth control.

```typescript
{
  dirPath: string;      // Required: relative path
  depth?: number;       // Default: 4
  includeHidden?: boolean;  // Default: false
  maxEntries?: number;  // Default: 2000
}
```

**Returns:** `{ entries: Array<{path, entryType, size?}>, truncated }`

---

### search_text

Search files using ripgrep.

```typescript
{
  pattern: string;      // Required: regex pattern
  cwd?: string;         // Default: "."
  globs?: string[];     // e.g., ["*.ts", "*.rs"]
  caseSensitive?: boolean;  // Default: false
  regex?: boolean;      // Default: true
  maxMatches?: number;  // Default: 200
  maxPerFile?: number;  // Default: 50
}
```

**Returns:** `{ matches: Array<{filePath, line, column?, text}>, truncated }`

---

### read_excerpt

Read specific line range from file.

```typescript
{
  filePath: string;     // Required: relative path
  startLine: number;    // Required: 1-based inclusive
  endLine: number;      // Required: 1-based inclusive
  maxBytes?: number;    // Default: 20000
}
```

**Returns:** `{ filePath, startLine, endLine, text, truncated }`

---

### explore_code

Extract symbols using tree-sitter.

```typescript
{
  filePath: string;     // Required: relative path
  exported: boolean;    // Required: filter exported only
  maxSymbols?: number;  // Default: 200
}
```

**Returns:** `{ filePath, language, symbols: Array<{name, kind, exported, location}>, notes? }`

**Supported:** `.ts`, `.tsx`, `.js`, `.jsx`, `.rs`, `.java`

---

### apply_patch

Apply anchor-based edits.

```typescript
{
  filePath: string;     // Required: relative path
  mode?: "strict" | "best_effort";  // Default: "strict"
  dryRun?: boolean;     // Default: false
  edits: Array<PatchEdit>;  // Required
}
```

**Edit Types:**

**Insert:**
```typescript
{
  kind: "insert",
  where: "before" | "after",
  anchor: { needle: string, requireUnique?: boolean },
  text: string
}
```

**Replace:**
```typescript
{
  kind: "replace",
  target: {
    start: { needle: string, requireUnique?: boolean },
    end?: { needle: string, requireUnique?: boolean },
    includeEndNeedle?: boolean
  },
  replacement: string
}
```

**Delete:**
```typescript
{
  kind: "delete",
  target: {
    start: { needle: string, requireUnique?: boolean },
    end?: { needle: string, requireUnique?: boolean },
    includeEndNeedle?: boolean
  }
}
```

**Returns:** `{ filePath, dryRun, applied: Array<{kind, summary, startLine?, endLine?}> }`

---

## Error Codes

| Code | Meaning | Action |
|------|---------|--------|
| `INVALID_ARGUMENT` | Bad input parameters | Check request format |
| `NOT_FOUND` | File/dir doesn't exist | Verify path |
| `PATH_OUTSIDE_ROOT` | Path traversal attempt | Use relative paths only |
| `DRIFT_DETECTED` | Anchor not found | Re-read file, update anchors |
| `SEARCH_FAILED` | ripgrep failed | Check pattern, ensure rg installed |
| `PARSE_FAILED` | tree-sitter failed | Check file syntax, language support |
| `IO_ERROR` | Filesystem error | Check permissions, disk space |
| `INTERNAL` | Unexpected error | Report bug |

---

## Common Patterns

### Find → Read → Edit

```javascript
// 1. Find
search_text({ pattern: "oldFunction" })

// 2. Read context
read_excerpt({ filePath: "...", startLine: 10, endLine: 30 })

// 3. Edit
apply_patch({
  filePath: "...",
  edits: [{ kind: "replace", target: {...}, replacement: "..." }]
})
```

### Explore API Surface

```javascript
// List modules
list_dir({ dirPath: "src/api", depth: 2 })

// Extract exports
explore_code({ filePath: "src/api/users.ts", exported: true })

// Read implementation
read_excerpt({ filePath: "src/api/users.ts", startLine: 10, endLine: 50 })
```

### Batch Edits

```javascript
apply_patch({
  filePath: "...",
  edits: [
    { kind: "insert", ... },
    { kind: "replace", ... },
    { kind: "delete", ... }
  ]
})
```

---

## Best Practices

✅ **DO:**
- Use `requireUnique: true` for anchors
- Run `dryRun: true` first for complex changes
- Scope searches with `cwd` and `globs`
- Handle `DRIFT_DETECTED` by re-reading
- Use unique, stable anchor text

❌ **DON'T:**
- Use line numbers (they change) - use anchors
- Ignore error codes
- Apply patches without reading first
- Use vague anchors ("}", "//")
- Traverse with `..` in paths

---

## Symbol Kinds by Language

### TypeScript/JavaScript
- `function` - Function declarations
- `class` - Class declarations
- `interface` - TypeScript interfaces
- `type` - Type aliases
- `enum` - Enumerations
- `const` - Constant declarations
- `variable` - Variable declarations

### Rust
- `function` - Functions
- `struct` - Struct definitions
- `enum` - Enum definitions
- `trait` - Trait definitions
- `module` - Module declarations
- `const` - Constants
- `variable` - Static/let bindings

### Java
- `class` - Class declarations
- `interface` - Interface declarations
- `enum` - Enum declarations
- `method` - Method definitions
- `field` - Field declarations

---

## Cache Behavior

**Automatic invalidation:**
- Cache checked on every file operation
- Compares (size, mtime) with disk
- Reloads if changed
- No manual cache management needed

**Benefits:**
- External edits (IDE, git) detected instantly
- No stale reads
- LLM always sees current state

---

## Testing

```bash
# Direct engine test
echo '{"id":"1","op":"list_dir","args":{"dirPath":"."}}' | \
  ./code-engine/target/release/code-engine .

# Full test suite
./test-mcp.sh
```

---

## Troubleshooting

**"Engine not found"**
```bash
cd code-engine && cargo build --release
```

**"ripgrep not found"**
```bash
brew install ripgrep  # macOS
apt install ripgrep   # Linux
```

**"Parse failed" for .ts files**
- Check file is valid TypeScript
- Ensure no syntax errors
- Try with simpler file first

**"Anchor not found"**
- File changed since last read
- Use `read_excerpt` to get current content
- Update anchor to match current text

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `REPO_ROOT` | Root directory to operate on | Current directory |
| `CODE_ENGINE_PATH` | Path to Rust binary | `../code-engine/target/release/code-engine` |

---

## Performance Tuning

**Large codebases:**
- Use specific `globs` in search
- Limit `depth` in list_dir
- Set appropriate `maxMatches`/`maxEntries`

**Memory:**
- Cache is per-process, cleared on restart
- Each file cached as Ropey structure
- LRU eviction not yet implemented

**Speed:**
- First read is slowest (parse + cache)
- Subsequent reads are fast (cached)
- ripgrep is extremely fast even on large repos

---

## Architecture Quick View

```
┌─────────────┐ MCP      ┌──────────────┐
│ LLM Client  │◄────────►│ Node Server  │
└─────────────┘ Protocol └──────────────┘
                                │
                          NDJSON│stdin/stdout
                                ▼
                         ┌──────────────┐
                         │ Rust Engine  │
                         │              │
                         │ - Ropey      │
                         │ - tree-sitter│
                         │ - ripgrep    │
                         │ - Cache      │
                         └──────────────┘
                                │
                                ▼
                          Filesystem
```

---

## Version Information

Check versions:
```bash
# Rust engine
./code-engine/target/release/code-engine --version

# Node server (in package.json)
cd mcp-server && npm version

# Dependencies
cargo tree  # Rust
npm list    # Node
```

---

## Links

- **Full Documentation:** `README.md`
- **Usage Examples:** `EXAMPLES.md`
- **MCP Config Example:** `mcp-config-example.json`
- **Test Script:** `test-mcp.sh`
