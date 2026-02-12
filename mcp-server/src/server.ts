import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";
import { RustEngineClient } from "./engineClient.js";
import { resolve } from "node:path";
import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Get repo root from environment or command line
const repoRoot = process.env.REPO_ROOT || process.argv[2] || process.cwd();
const enginePath =
  process.env.CODE_ENGINE_PATH ||
  resolve(__dirname, "../../code-engine/target/release/code-engine");
const dockerImage = process.env.AGENT_MCP_DOCKER_IMAGE || "ubuntu:24.04";

// Initialize Rust engine client
const engine = new RustEngineClient({
  cmd: enginePath,
  args: [repoRoot],
});

// Create MCP server
const server = new Server(
  {
    name: "agent-mcp-server",
    version: "0.1.0",
  },
  {
    capabilities: {
      tools: {},
    },
  },
);

// Define tools
const tools: Tool[] = [
  {
    name: "list_dir",
    description:
      "List files and directories in a given path with configurable depth and filtering",
    inputSchema: {
      type: "object",
      properties: {
        dirPath: {
          type: "string",
          description: "Relative path to directory within repo root",
        },
        depth: {
          type: "number",
          description: "Maximum depth to traverse (default: 1)",
          default: 1,
        },
        includeHidden: {
          type: "boolean",
          description: "Include hidden files (default: false)",
          default: false,
        },
        maxEntries: {
          type: "number",
          description: "Maximum number of entries to return (default: 2000)",
          default: 2000,
        },
      },
      required: ["dirPath"],
    },
  },
  {
    name: "search_text",
    description:
      "Search for text patterns in files using ripgrep with regex support",
    inputSchema: {
      type: "object",
      properties: {
        pattern: {
          type: "string",
          description: "Search pattern (regex by default)",
        },
        cwd: {
          type: "string",
          description: "Working directory relative to repo root (default: '.')",
        },
        globs: {
          type: "array",
          items: { type: "string" },
          description: "File glob patterns to include (e.g., ['*.ts', '*.rs'])",
          default: [],
        },
        caseSensitive: {
          type: "boolean",
          description: "Case-sensitive search (default: false)",
        },
        regex: {
          type: "boolean",
          description: "Treat pattern as regex (default: true)",
        },
        noIgnore: {
          type: "boolean",
          description: "Ignore .gitignore and search all files (default: true)",
          default: true,
        },
        maxMatches: {
          type: "number",
          description: "Maximum total matches to return (default: 200)",
          default: 200,
        },
        maxPerFile: {
          type: "number",
          description: "Maximum matches per file (default: 50)",
          default: 50,
        },
        maxLineLength: {
          type: "number",
          description:
            "Maximum line length in characters. Lines longer than this will be truncated with context around the match (default: 200)",
          default: 200,
        },
      },
      required: ["pattern"],
    },
  },
  {
    name: "read_excerpt",
    description:
      "Read a specific line range from a file with byte limit enforcement",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file within repo root",
        },
        startLine: {
          type: "number",
          description: "Starting line number (1-based, inclusive)",
        },
        endLine: {
          type: "number",
          description: "Ending line number (1-based, inclusive)",
        },
        maxBytes: {
          type: "number",
          description:
            "Maximum bytes to return (default: 20000, hard limit: 102400 = 100KB). Values above 100KB will be capped to 100KB.",
          default: 20000,
          maximum: 102400,
        },
      },
      required: ["filePath", "startLine", "endLine"],
    },
  },
  {
    name: "explore_code",
    description:
      "Extract code symbols (functions, classes, etc.) from a file using tree-sitter",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file within repo root",
        },
        exported: {
          type: "boolean",
          description: "Filter to only exported symbols",
        },
        maxSymbols: {
          type: "number",
          description: "Maximum number of symbols to return (default: 200)",
          default: 200,
        },
      },
      required: ["filePath", "exported"],
    },
  },
  {
    name: "apply_patch",
    description:
      "Apply anchor-based edits to a file with strict drift detection",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file within repo root",
        },
        mode: {
          type: "string",
          enum: ["strict", "best_effort"],
          description:
            "Edit mode: strict fails on any drift, best_effort continues (default: strict)",
        },
        dryRun: {
          type: "boolean",
          description: "Preview changes without writing (default: false)",
          default: false,
        },
        edits: {
          type: "array",
          description: "Array of edit operations to apply",
          items: {
            type: "object",
            oneOf: [
              {
                type: "object",
                properties: {
                  kind: { type: "string", const: "insert" },
                  where: {
                    type: "string",
                    enum: ["before", "after"],
                    description: "Insert before or after anchor",
                  },
                  anchor: {
                    type: "object",
                    properties: {
                      needle: { type: "string" },
                      requireUnique: { type: "boolean" },
                    },
                    required: ["needle"],
                  },
                  text: { type: "string" },
                },
                required: ["kind", "where", "anchor", "text"],
              },
              {
                type: "object",
                properties: {
                  kind: { type: "string", const: "replace" },
                  target: {
                    type: "object",
                    properties: {
                      start: {
                        type: "object",
                        properties: {
                          needle: { type: "string" },
                          requireUnique: { type: "boolean" },
                        },
                        required: ["needle"],
                      },
                      end: {
                        type: "object",
                        properties: {
                          needle: { type: "string" },
                          requireUnique: { type: "boolean" },
                        },
                        required: ["needle"],
                      },
                      includeEndNeedle: { type: "boolean" },
                    },
                    required: ["start"],
                  },
                  replacement: { type: "string" },
                },
                required: ["kind", "target", "replacement"],
              },
              {
                type: "object",
                properties: {
                  kind: { type: "string", const: "delete" },
                  target: {
                    type: "object",
                    properties: {
                      start: {
                        type: "object",
                        properties: {
                          needle: { type: "string" },
                          requireUnique: { type: "boolean" },
                        },
                        required: ["needle"],
                      },
                      end: {
                        type: "object",
                        properties: {
                          needle: { type: "string" },
                          requireUnique: { type: "boolean" },
                        },
                        required: ["needle"],
                      },
                      includeEndNeedle: { type: "boolean" },
                    },
                    required: ["start"],
                  },
                },
                required: ["kind", "target"],
              },
            ],
          },
        },
      },
      required: ["filePath", "edits"],
    },
  },
  {
    name: "create_directory",
    description:
      "Create a new directory at the specified path, creating parent directories as needed",
    inputSchema: {
      type: "object",
      properties: {
        dirPath: {
          type: "string",
          description: "Relative path to directory within repo root",
        },
      },
      required: ["dirPath"],
    },
  },
  {
    name: "create_file",
    description:
      "Create a new file with the specified contents, creating parent directories as needed",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file within repo root",
        },
        contents: {
          type: "string",
          description: "File contents to write",
        },
        encoding: {
          type: "string",
          description: "File encoding (default: utf8)",
          default: "utf8",
        },
      },
      required: ["filePath", "contents"],
    },
  },
  {
    name: "read_file_info",
    description:
      "Get metadata about a file or directory (size, mtime, type, existence)",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file or directory within repo root",
        },
      },
      required: ["filePath"],
    },
  },
  {
    name: "delete_file",
    description:
      "Delete a file or directory (recursively) at the specified path",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file or directory within repo root",
        },
      },
      required: ["filePath"],
    },
  },
  {
    name: "read_file",
    description: "Read entire file contents with a hard limit of 100KB",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Relative path to file within repo root",
        },
        maxBytes: {
          type: "number",
          description:
            "Maximum bytes to read (default and hard limit: 102400 = 100KB). Values above 100KB will be capped to 100KB.",
          default: 102400,
          maximum: 102400,
        },
      },
      required: ["filePath"],
    },
  },
  {
    name: "docker_shell",
    description: `Execute a shell command in an isolated ${dockerImage} Docker container with the repo root mounted as the working directory. Provides a consistent, sandboxed execution environment regardless of host OS. Returns output as an array of [data, stream] tuples that preserve temporal ordering of stdout and stderr.`,
    inputSchema: {
      type: "object",
      properties: {
        command: {
          type: "string",
          description: "The shell command to execute in the container",
        },
        timeoutMs: {
          type: "number",
          description:
            "Maximum execution time in milliseconds (default: 30000)",
          default: 30000,
        },
        workDir: {
          type: "string",
          description:
            "Working directory relative to repo root (default: '.' for repo root)",
          default: ".",
        },
      },
      required: ["command"],
    },
  },
];

// Register tool list handler
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return { tools };
});

// Register tool call handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    let result: any;

    switch (name) {
      case "list_dir":
        result = await engine.listDir(args);
        break;

      case "search_text":
        result = await engine.searchText(args);
        break;

      case "read_excerpt":
        result = await engine.readExcerpt(args);
        break;

      case "explore_code":
        result = await engine.exploreCode(args);
        break;

      case "apply_patch":
        result = await engine.applyPatch(args);
        break;

      case "create_directory":
        result = await engine.call("create_directory", args);
        break;

      case "create_file":
        result = await engine.call("create_file", args);
        break;

      case "read_file_info":
        result = await engine.call("read_file_info", args);
        break;

      case "delete_file":
        result = await engine.call("delete_file", args);
        break;

      case "read_file":
        result = await engine.call("read_file", args);
        break;

      case "docker_shell":
        result = await executeDockerShell(args);
        break;

      default:
        return {
          content: [
            {
              type: "text",
              text: `Unknown tool: ${name}`,
            },
          ],
          isError: true,
        };
    }

    // Return successful result
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(result, null, 2),
        },
      ],
    };
  } catch (error: any) {
    // Map engine errors to MCP error responses
    const engineError = error.engineError;
    if (engineError) {
      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                error: engineError.code,
                message: engineError.message,
                details: engineError.details,
              },
              null,
              2,
            ),
          },
        ],
        isError: true,
      };
    }

    // Generic error
    return {
      content: [
        {
          type: "text",
          text: `Error: ${error.message}`,
        },
      ],
      isError: true,
    };
  }
});

// Persistent Docker container management
let dockerContainerId: string | null = null;

async function ensureDockerContainer(): Promise<string> {
  if (dockerContainerId) return dockerContainerId;

  const containerName = `agent-mcp-${randomBytes(8).toString("hex")}`;
  const uid = process.getuid?.() || 1000;
  const gid = process.getgid?.() || 1000;

  return new Promise((resolve, reject) => {
    const proc = spawn("docker", [
      "run",
      "-d",
      "--rm",
      "--name",
      containerName,
      "--user",
      `${uid}:${gid}`,
      "-v",
      `${repoRoot}:/workspace`,
      "-w",
      "/workspace",
      dockerImage,
      "sleep",
      "infinity",
    ]);

    let output = "";
    proc.stdout.on("data", (chunk) => (output += chunk.toString()));
    proc.stderr.on("data", (chunk) => (output += chunk.toString()));

    proc.on("close", (code) => {
      if (code === 0) {
        dockerContainerId = output.trim();
        resolve(dockerContainerId!);
      } else {
        reject(new Error(`Failed to start Docker container: ${output}`));
      }
    });
  });
}

function stopDockerContainer() {
  if (!dockerContainerId) return;
  spawn("docker", ["stop", dockerContainerId], { stdio: "ignore" });
  dockerContainerId = null;
}

process.on("exit", stopDockerContainer);
process.on("SIGINT", () => {
  stopDockerContainer();
  process.exit(0);
});
process.on("SIGTERM", () => {
  stopDockerContainer();
  process.exit(0);
});

// Docker shell execution
async function executeDockerShell(args: any): Promise<any> {
  const command = args.command;
  const timeoutMs = args.timeoutMs || 30000;
  const workDir = args.workDir || ".";

  if (!command || typeof command !== "string") {
    throw new Error("command must be a non-empty string");
  }

  const containerId = await ensureDockerContainer();

  const dockerArgs = [
    "exec",
    "-w",
    `/workspace/${workDir}`,
    containerId,
    "/bin/bash",
    "-c",
    command,
  ];

  return new Promise((resolve, reject) => {
    const output: Array<[string, "stdout" | "stderr"]> = [];
    let timedOut = false;

    const proc = spawn("docker", dockerArgs, {
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });

    const timer = setTimeout(() => {
      timedOut = true;
      proc.kill("SIGTERM");
      setTimeout(() => proc.kill("SIGKILL"), 2000);
    }, timeoutMs);

    proc.stdout.on("data", (chunk) => {
      output.push([chunk.toString(), "stdout"]);
    });

    proc.stderr.on("data", (chunk) => {
      output.push([chunk.toString(), "stderr"]);
    });

    proc.on("error", (error) => {
      clearTimeout(timer);
      reject(
        new Error(
          `Failed to exec in Docker container: ${error.message}. Ensure Docker is running.`,
        ),
      );
    });

    proc.on("close", (code, signal) => {
      clearTimeout(timer);

      if (timedOut) {
        const partialOutput = output.map(([data]) => data).join("");
        reject(
          new Error(
            `Command timed out after ${timeoutMs}ms. Partial output:\n${partialOutput}`,
          ),
        );
        return;
      }

      resolve({
        exitCode: code ?? (signal ? -1 : 0),
        signal: signal || null,
        output,
        success: code === 0,
      });
    });
  });
}

// Start the server
async function main() {
  console.error(`Starting agent-mcp-server for repo: ${repoRoot}`);
  console.error(`Using Rust engine: ${enginePath}`);

  // Start the Rust engine
  engine.start();

  // Connect via stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error("MCP server running on stdio");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
