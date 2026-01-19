# Usage Examples

This document provides practical examples of using the Agent MCP Server tools.

## Setup

First, ensure the server is running:

```bash
cd mcp-server
export REPO_ROOT=/path/to/your/codebase
npm start
```

Or configure it in your MCP client configuration file.

## Tool Examples

### 1. list_dir - Browse Directory Structure

List files in a directory with configurable depth:

**Request:**
```json
{
  "name": "list_dir",
  "arguments": {
    "dirPath": "src",
    "depth": 2,
    "includeHidden": false,
    "maxEntries": 100
  }
}
```

**Response:**
```json
{
  "entries": [
    {"path": "components", "entryType": "dir"},
    {"path": "components/Button.tsx", "entryType": "file", "size": 1234},
    {"path": "components/Input.tsx", "entryType": "file", "size": 2345},
    {"path": "utils", "entryType": "dir"},
    {"path": "utils/helpers.ts", "entryType": "file", "size": 3456}
  ],
  "truncated": false
}
```

**Use Cases:**
- Explore unfamiliar codebases
- Find specific directories
- Get project structure overview
- Locate configuration files

---

### 2. search_text - Find Code Patterns

Search for patterns across multiple files using regex:

**Example 1: Find all function declarations**

```json
{
  "name": "search_text",
  "arguments": {
    "pattern": "function\\s+\\w+",
    "cwd": "src",
    "globs": ["*.ts", "*.tsx"],
    "caseSensitive": false,
    "regex": true,
    "maxMatches": 50
  }
}
```

**Example 2: Find TODO comments**

```json
{
  "name": "search_text",
  "arguments": {
    "pattern": "TODO|FIXME|XXX",
    "cwd": ".",
    "regex": true,
    "maxMatches": 100
  }
}
```

**Example 3: Find imports of a specific module**

```json
{
  "name": "search_text",
  "arguments": {
    "pattern": "import.*from ['\"]react['\"]",
    "globs": ["*.tsx", "*.jsx"],
    "maxMatches": 200
  }
}
```

**Response:**
```json
{
  "matches": [
    {
      "filePath": "src/components/Button.tsx",
      "line": 1,
      "column": 1,
      "text": "import React from 'react';"
    },
    {
      "filePath": "src/components/Input.tsx",
      "line": 1,
      "column": 1,
      "text": "import React from 'react';"
    }
  ],
  "truncated": false
}
```

**Use Cases:**
- Find all usages of a function or class
- Locate TODO comments
- Search for specific patterns
- Find API endpoints
- Identify deprecated code

---

### 3. read_excerpt - Read Specific File Sections

Read specific line ranges from files:

**Example 1: Read function implementation**

```json
{
  "name": "read_excerpt",
  "arguments": {
    "filePath": "src/utils/helpers.ts",
    "startLine": 45,
    "endLine": 65,
    "maxBytes": 10000
  }
}
```

**Example 2: Read file header**

```json
{
  "name": "read_excerpt",
  "arguments": {
    "filePath": "src/index.ts",
    "startLine": 1,
    "endLine": 20
  }
}
```

**Response:**
```json
{
  "filePath": "src/utils/helpers.ts",
  "startLine": 45,
  "endLine": 65,
  "text": "export function formatDate(date: Date): string {\n  const year = date.getFullYear();\n  const month = String(date.getMonth() + 1).padStart(2, '0');\n  const day = String(date.getDate()).padStart(2, '0');\n  return `${year}-${month}-${day}`;\n}\n\nexport function isValidEmail(email: string): boolean {\n  const regex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;\n  return regex.test(email);\n}",
  "truncated": false
}
```

**Use Cases:**
- Read function implementations found via search
- Examine specific code sections
- Review configuration sections
- Check documentation
- Verify code before editing

---

### 4. explore_code - Extract Symbols from Files

Analyze code structure and extract symbols:

**Example 1: Find all exported functions in a TypeScript file**

```json
{
  "name": "explore_code",
  "arguments": {
    "filePath": "src/api/users.ts",
    "exported": true,
    "maxSymbols": 100
  }
}
```

**Response:**
```json
{
  "filePath": "src/api/users.ts",
  "language": "ts",
  "symbols": [
    {
      "name": "UserService",
      "kind": "class",
      "exported": true,
      "location": {"startLine": 10, "endLine": 50}
    },
    {
      "name": "getUser",
      "kind": "function",
      "exported": true,
      "location": {"startLine": 52, "endLine": 60}
    },
    {
      "name": "createUser",
      "kind": "function",
      "exported": true,
      "location": {"startLine": 62, "endLine": 75}
    },
    {
      "name": "User",
      "kind": "interface",
      "exported": true,
      "location": {"startLine": 5, "endLine": 8}
    }
  ]
}
```

**Example 2: Explore all symbols (public and private) in Rust**

```json
{
  "name": "explore_code",
  "arguments": {
    "filePath": "src/lib.rs",
    "exported": false,
    "maxSymbols": 200
  }
}
```

**Supported Languages:**
- TypeScript/JavaScript: functions, classes, interfaces, types, enums, constants
- Rust: functions, structs, enums, traits, modules, constants
- Java: classes, interfaces, enums, methods, fields

**Use Cases:**
- Get API overview
- Find available functions/classes
- Understand module structure
- Identify entry points
- Generate documentation

---

### 5. apply_patch - Intelligent Code Editing

Apply anchor-based edits to files with drift detection:

**Example 1: Add an import statement**

```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/components/Button.tsx",
    "mode": "strict",
    "dryRun": false,
    "edits": [
      {
        "kind": "insert",
        "where": "after",
        "anchor": {
          "needle": "import React from 'react';",
          "requireUnique": true
        },
        "text": "\nimport { styled } from 'styled-components';"
      }
    ]
  }
}
```

**Example 2: Replace a function implementation**

```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/utils/validation.ts",
    "mode": "strict",
    "edits": [
      {
        "kind": "replace",
        "target": {
          "start": {"needle": "export function isValidEmail(email: string) {"},
          "end": {"needle": "}", "includeEndNeedle": true}
        },
        "replacement": "export function isValidEmail(email: string): boolean {\n  const regex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;\n  return regex.test(email);\n}"
      }
    ]
  }
}
```

**Example 3: Delete a deprecated function**

```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/utils/legacy.ts",
    "mode": "strict",
    "edits": [
      {
        "kind": "delete",
        "target": {
          "start": {"needle": "// @deprecated"},
          "end": {"needle": "}", "includeEndNeedle": true}
        }
      }
    ]
  }
}
```

**Example 4: Multiple edits in one operation**

```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/index.ts",
    "mode": "strict",
    "dryRun": true,
    "edits": [
      {
        "kind": "insert",
        "where": "after",
        "anchor": {"needle": "import express from 'express';"},
        "text": "\nimport cors from 'cors';"
      },
      {
        "kind": "insert",
        "where": "before",
        "anchor": {"needle": "app.listen(3000);"},
        "text": "app.use(cors());\n"
      }
    ]
  }
}
```

**Response:**
```json
{
  "filePath": "src/index.ts",
  "dryRun": true,
  "applied": [
    {
      "kind": "insert",
      "summary": "Insert 23 bytes after anchor",
      "startLine": 1,
      "endLine": 1
    },
    {
      "kind": "insert",
      "summary": "Insert 17 bytes before anchor",
      "startLine": 15,
      "endLine": 15
    }
  ]
}
```

**Modes:**
- `strict` (default): Fails if any anchor is not found or is ambiguous
- `best_effort`: Continues if anchors are missing (use with caution)

**Features:**
- **Anchor-based**: Edit by finding text, not line numbers
- **Drift detection**: Fails if file changed since you last read it
- **Dry run**: Preview changes before applying
- **Atomic writes**: Uses temp file + rename for safety
- **Multiple edits**: Apply several changes in one operation

**Use Cases:**
- Add imports/dependencies
- Refactor function implementations
- Update configuration
- Remove deprecated code
- Fix bugs identified by search

---

## Workflow Examples

### Workflow 1: Find and Fix a Bug

1. **Search for problematic pattern:**
```json
{"name": "search_text", "arguments": {"pattern": "console\\.log"}}
```

2. **Read the context:**
```json
{"name": "read_excerpt", "arguments": {"filePath": "src/debug.ts", "startLine": 10, "endLine": 30}}
```

3. **Remove debug statements:**
```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/debug.ts",
    "edits": [
      {"kind": "delete", "target": {"start": {"needle": "console.log('Debug:"}}}
    ]
  }
}
```

### Workflow 2: Add a New Feature

1. **Explore existing API:**
```json
{"name": "explore_code", "arguments": {"filePath": "src/api/index.ts", "exported": true}}
```

2. **Read example implementation:**
```json
{"name": "read_excerpt", "arguments": {"filePath": "src/api/users.ts", "startLine": 1, "endLine": 50}}
```

3. **Add new endpoint:**
```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/api/index.ts",
    "edits": [
      {
        "kind": "insert",
        "where": "after",
        "anchor": {"needle": "app.use('/users', userRouter);"},
        "text": "\napp.use('/posts', postRouter);"
      }
    ]
  }
}
```

### Workflow 3: Refactor Code

1. **Find all usages:**
```json
{"name": "search_text", "arguments": {"pattern": "oldFunctionName"}}
```

2. **Update each file:**
```json
{
  "name": "apply_patch",
  "arguments": {
    "filePath": "src/utils/helper.ts",
    "edits": [
      {
        "kind": "replace",
        "target": {"start": {"needle": "function oldFunctionName"}, "end": {"needle": "}"}},
        "replacement": "function newFunctionName() {\n  // Updated implementation\n}"
      }
    ]
  }
}
```

---

## Error Handling

The server returns structured errors with specific error codes:

### NOT_FOUND
```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "File not found",
    "details": {"path": "missing.ts"}
  }
}
```

### PATH_OUTSIDE_ROOT
```json
{
  "error": {
    "code": "PATH_OUTSIDE_ROOT",
    "message": "Path traversal is not allowed",
    "details": {"path": "../../../etc/passwd"}
  }
}
```

### DRIFT_DETECTED
```json
{
  "error": {
    "code": "DRIFT_DETECTED",
    "message": "Anchor not found",
    "details": {
      "edit": 0,
      "anchor": "import React from 'react';"
    }
  }
}
```

When you encounter DRIFT_DETECTED:
1. Re-read the file with `read_excerpt`
2. Update your anchors based on current content
3. Retry the patch operation

---

## Best Practices

### 1. Always Read Before Editing
```
search_text → read_excerpt → apply_patch
```

### 2. Use Dry Run for Complex Changes
```json
{"dryRun": true}  // Preview changes first
```

### 3. Use Unique Anchors
```json
{"anchor": {"needle": "exact unique text", "requireUnique": true}}
```

### 4. Handle Errors Gracefully
- Check error codes
- Re-read files if drift detected
- Use search to find new anchors

### 5. Batch Related Edits
Apply multiple related edits in one `apply_patch` call to maintain consistency.

### 6. Scope Your Searches
Use `cwd` and `globs` to narrow search results:
```json
{"cwd": "src/components", "globs": ["*.tsx"]}
```

---

## Performance Tips

- **list_dir**: Use appropriate `depth` and `maxEntries` limits
- **search_text**: Use `globs` to filter file types
- **read_excerpt**: Request only the lines you need
- **explore_code**: Set reasonable `maxSymbols` limits
- **apply_patch**: Group related edits to reduce cache invalidation

---

## Advanced Usage

### Combining Tools for Complex Tasks

**Task: Add error handling to all async functions**

1. Find all async functions:
```json
{"name": "search_text", "arguments": {"pattern": "async function"}}
```

2. For each result, read the function:
```json
{"name": "read_excerpt", "arguments": {"filePath": "...", "startLine": 10, "endLine": 30}}
```

3. Add try-catch wrapper:
```json
{
  "name": "apply_patch",
  "arguments": {
    "edits": [
      {"kind": "insert", "where": "after", "anchor": {"needle": "async function processData() {"}, "text": "\n  try {"},
      {"kind": "insert", "where": "before", "anchor": {"needle": "}"}, "text": "  } catch (error) {\n    console.error(error);\n    throw error;\n  }\n"}
    ]
  }
}
```

---

## Troubleshooting

### Issue: "Anchor not found"
- File may have changed externally
- Re-read the file to get current content
- Update your anchor strings

### Issue: "Path outside root"
- All paths must be relative to `REPO_ROOT`
- No `..` traversal allowed
- Check your `dirPath`/`filePath` arguments

### Issue: "Parse failed"
- File may not be valid UTF-8
- Language not supported for `explore_code`
- File may be too large or malformed

### Issue: Search returns no results
- Check regex syntax
- Try case-insensitive search
- Verify `cwd` and `globs` filters
- Ensure ripgrep is installed