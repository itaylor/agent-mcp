#!/bin/bash

set -e

echo "========================================"
echo "Testing Agent MCP Server"
echo "========================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Check if Rust engine is built
if [ ! -f "code-engine/target/release/code-engine" ]; then
    echo -e "${RED}Error: Rust engine not built${NC}"
    echo "Run: cd code-engine && cargo build --release"
    exit 1
fi

# Check if Node server is built
if [ ! -d "mcp-server/dist" ]; then
    echo -e "${RED}Error: Node server not built${NC}"
    echo "Run: cd mcp-server && npm install && npm run build"
    exit 1
fi

ENGINE="./code-engine/target/release/code-engine"

echo "========================================"
echo "Test 1: list_dir"
echo "========================================"
REQUEST='{"id":"test1","op":"list_dir","args":{"dirPath":".","depth":1,"maxEntries":10}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 2: read_excerpt"
echo "========================================"
REQUEST='{"id":"test2","op":"read_excerpt","args":{"filePath":"README.md","startLine":1,"endLine":5}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 3: explore_code (TypeScript)"
echo "========================================"
REQUEST='{"id":"test3","op":"explore_code","args":{"filePath":"mcp-server/src/engineClient.ts","exported":true,"maxSymbols":5}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 4: search_text (if ripgrep available)"
echo "========================================"
if command -v rg &> /dev/null; then
    REQUEST='{"id":"test4","op":"search_text","args":{"pattern":"function","cwd":"mcp-server/src","maxMatches":5}}'
    echo "Request: $REQUEST"
    echo ""
    RESPONSE=$(echo "$REQUEST" | $ENGINE .)
    echo "Response:"
    echo "$RESPONSE" | jq '.'
    if echo "$RESPONSE" | jq -e '.ok == true' > /dev/null; then
        echo -e "${GREEN}✓ Test passed${NC}"
    else
        echo -e "${RED}✗ Test failed${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ Skipping (ripgrep not installed)${NC}"
fi
echo ""

echo "========================================"
echo "Test 5: Error handling (invalid path)"
echo "========================================"
REQUEST='{"id":"test5","op":"read_excerpt","args":{"filePath":"nonexistent.txt","startLine":1,"endLine":5}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == false and .error.code == "NOT_FOUND"' > /dev/null; then
    echo -e "${GREEN}✓ Test passed (error correctly returned)${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 6: Path sandboxing (security)"
echo "========================================"
REQUEST='{"id":"test6","op":"read_excerpt","args":{"filePath":"../etc/passwd","startLine":1,"endLine":5}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == false and .error.code == "PATH_OUTSIDE_ROOT"' > /dev/null; then
    echo -e "${GREEN}✓ Test passed (path traversal blocked)${NC}"
else
    echo -e "${RED}✗ Test failed (path traversal not blocked!)${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 7: create_directory"
echo "========================================"
REQUEST='{"id":"test7","op":"create_directory","args":{"dirPath":"test_temp_dir/nested/deep"}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true and .result.created == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
    # Verify directory was actually created
    if [ -d "test_temp_dir/nested/deep" ]; then
        echo -e "${GREEN}✓ Directory verified on filesystem${NC}"
    else
        echo -e "${RED}✗ Directory not found on filesystem${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 8: create_file"
echo "========================================"
REQUEST='{"id":"test8","op":"create_file","args":{"filePath":"test_temp_dir/test_file.txt","contents":"Test content\n","encoding":"utf8"}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true and .result.created == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
    # Verify file was created with correct content
    if [ -f "test_temp_dir/test_file.txt" ]; then
        CONTENT=$(cat test_temp_dir/test_file.txt)
        if [ "$CONTENT" = "Test content" ]; then
            echo -e "${GREEN}✓ File content verified${NC}"
        else
            echo -e "${RED}✗ File content incorrect${NC}"
            exit 1
        fi
    else
        echo -e "${RED}✗ File not found on filesystem${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 9: read_file_info"
echo "========================================"
REQUEST='{"id":"test9","op":"read_file_info","args":{"filePath":"test_temp_dir/test_file.txt"}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true and .result.exists == true and .result.entryType == "file"' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 10: read_file"
echo "========================================"
REQUEST='{"id":"test10","op":"read_file","args":{"filePath":"test_temp_dir/test_file.txt"}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true and (.result.contents | contains("Test content"))' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo "Test 11: delete_file"
echo "========================================"
REQUEST='{"id":"test11","op":"delete_file","args":{"filePath":"test_temp_dir"}}'
echo "Request: $REQUEST"
echo ""
RESPONSE=$(echo "$REQUEST" | $ENGINE .)
echo "Response:"
echo "$RESPONSE" | jq '.'
if echo "$RESPONSE" | jq -e '.ok == true and .result.deleted == true' > /dev/null; then
    echo -e "${GREEN}✓ Test passed${NC}"
    # Verify directory was actually deleted
    if [ ! -d "test_temp_dir" ]; then
        echo -e "${GREEN}✓ Directory deletion verified${NC}"
    else
        echo -e "${RED}✗ Directory still exists${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Test failed${NC}"
    exit 1
fi
echo ""

echo "========================================"
echo -e "${GREEN}All tests passed!${NC}"
echo "========================================"
echo ""
echo "To run the MCP server:"
echo "  cd mcp-server"
echo "  REPO_ROOT=/path/to/your/repo npm start"
echo ""
echo "Or use in MCP client configuration:"
echo '  {'
echo '    "command": "node",'
echo '    "args": ["'$SCRIPT_DIR'/mcp-server/dist/index.js"],'
echo '    "env": { "REPO_ROOT": "/path/to/repo" }'
echo '  }'
