# TODO

This document tracks known issues, planned improvements, and missing features for the Agent MCP Server.

## Critical Issues

### 1. Cache Unbounded Growth (Memory Leak)

**Location**: `code-engine/src/cache.rs`

**Problem**: The Ropey cache (`HashMap<PathBuf, CacheEntry>`) grows unbounded without any eviction policy. In long-running sessions that access hundreds or thousands of files, memory consumption will grow without limit until OOM.

**Impact**: High - Will crash in production during long agent sessions

**Solution**: Implement LRU (Least Recently Used) eviction:
- Add configurable size limit (e.g., max 100 entries or 500MB total)
- Track access time for each cache entry
- Evict oldest entries when limit reached
- Consider using `lru` crate or similar

**Estimated Effort**: Medium (4-8 hours)

---

### 2. No Batch Operation Support

**Problem**: Every operation requires a separate request/response round-trip. Agents that need to read 10 files or apply patches to 5 files must make 10 or 5 separate requests.

**Impact**: High - Performance bottleneck for typical agent workflows

**Solution Options**:
1. Add `batch` operation that accepts array of operations and returns array of results
2. Add operation-specific batch variants (e.g., `read_files_batch`, `apply_patches_batch`)
3. Implement request pipelining in Node server

**Example**:
```json
{
  "op": "batch",
  "args": {
    "operations": [
      {"op": "read_file", "args": {...}},
      {"op": "apply_patch", "args": {...}},
      {"op": "read_excerpt", "args": {...}}
    ]
  }
}
```

**Estimated Effort**: Medium-Large (8-16 hours)

---

### 3. Engine Crash Recovery

**Problem**: If the Rust engine crashes, all in-flight requests fail and the Node server has no recovery mechanism. The entire MCP connection must be restarted.

**Impact**: Medium-High - Poor reliability in production

**Solution**:
- Add automatic engine restart in `engineClient.ts`
- Implement retry logic for failed requests (read-only ops only)
- Add health check mechanism
- Log crashes for debugging

**Considerations**:
- Don't retry mutating operations (apply_patch, delete_file, etc.)
- Add exponential backoff for repeated crashes
- Notify user if engine repeatedly fails

**Estimated Effort**: Medium (6-10 hours)

---

## Code Quality Issues

### 4. Incorrect Exported Detection in `explore_code`

**Location**: `code-engine/src/ops/explore_code.rs`

**Problem**: The `is_exported_ts`, `is_exported_rust`, and `is_exported_java` functions mark ALL methods/properties inside an exported class as `exported: true`. Only top-level exports should be marked as exported.

**Current Behavior**:
```typescript
export class Foo {
  bar() {}  // Currently marked as exported: true
}
```

**Expected Behavior**:
```typescript
export class Foo {     // exported: true (correct)
  bar() {}             // exported: false (member of exported class)
}

export function baz()  // exported: true (correct)
```

**Impact**: Medium - Agents may misunderstand API surface

**Solution**: Track export context during tree traversal. Only mark direct children of export statements as exported, not descendants.

**Estimated Effort**: Medium (4-6 hours per language)

---

## Nice-to-Have Features

### 5. `.gitignore` Support in `list_dir`

**Problem**: `list_dir` returns all files including `node_modules/`, `target/`, `.git/`, etc. Agents waste time exploring irrelevant directories.

**Solution**:
- Add optional `respectGitignore: boolean` parameter (default: true)
- Use `ignore` crate to parse `.gitignore` files
- Filter entries based on gitignore rules

**Estimated Effort**: Small (2-4 hours)

---

### 6. Return Modified Content from `apply_patch`

**Problem**: After applying a patch, agents must make a separate `read_file` or `read_excerpt` call to see the result. This doubles the round-trips for edit operations.

**Solution**:
- Add optional `returnContent: boolean` parameter to `apply_patch`
- Include `newContent?: string` in `ApplyPatchResult`
- Only return if requested (keep response size reasonable)

**Estimated Effort**: Small (1-2 hours)

---

### 7. Regex Anchor Support in `apply_patch`

**Problem**: Anchors only support exact string matching. Can't anchor on patterns like `function \w+