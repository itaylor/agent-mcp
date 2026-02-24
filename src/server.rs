use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    handler::server::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::{cache::RopeCache, ops, protocol::*, workspace::Workspace};

fn to_tool_result<T: serde::Serialize>(result: Result<T, EngineError>) -> CallToolResult {
    match result {
        Ok(r) => CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&r).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}")),
        )]),
        Err(e) => {
            let text = serde_json::to_string_pretty(&serde_json::json!({
                "error": e.code,
                "message": e.message,
                "details": e.details,
            }))
            .unwrap_or_else(|_| format!("{{\"error\": \"{}\"}}", e.message));
            CallToolResult {
                content: vec![Content::text(text)],
                is_error: Some(true),
                structured_content: None,
                meta: None,
            }
        }
    }
}

#[derive(Clone)]
pub struct AgentMcp {
    workspace: Arc<Workspace>,
    cache: Arc<Mutex<RopeCache>>,
    docker: Arc<ops::docker_shell::DockerManager>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AgentMcp {
    pub fn new(repo_root: &str) -> anyhow::Result<Self> {
        let docker_image =
            std::env::var("AGENT_MCP_DOCKER_IMAGE").unwrap_or_else(|_| "ubuntu:24.04".to_string());
        let abs_root = std::fs::canonicalize(repo_root)
            .map_err(|e| anyhow::anyhow!("Cannot resolve repo root '{}': {}", repo_root, e))?;

        Ok(Self {
            workspace: Arc::new(Workspace::new(repo_root.to_string())?),
            cache: Arc::new(Mutex::new(RopeCache::new())),
            docker: Arc::new(ops::docker_shell::DockerManager::new(
                abs_root.to_string_lossy().to_string(),
                docker_image,
            )),
            tool_router: Self::tool_router(),
        })
    }

    pub async fn shutdown(&self) {
        self.docker.stop().await;
    }

    #[tool(
        description = "List files and directories in a given path with configurable depth and filtering"
    )]
    async fn list_dir(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ListDirArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::list_dir::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Search for text patterns in files using ripgrep with regex support")]
    async fn search_text(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::search_text::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(
        description = "Read a specific line range from a file. maxBytes hard limit is 100KB (default 20KB)"
    )]
    async fn read_excerpt(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ReadExcerptArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut cache = self.cache.lock().await;
        let result = ops::read_excerpt::run(&self.workspace, &mut cache, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(
        description = "Extract code symbols (functions, classes, interfaces, etc.) from a file using tree-sitter"
    )]
    async fn explore_code(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ExploreCodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut cache = self.cache.lock().await;
        let result = ops::explore_code::run(&self.workspace, &mut cache, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Apply anchor-based edits to a file with strict drift detection")]
    async fn apply_patch(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ApplyPatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut cache = self.cache.lock().await;
        let result = ops::apply_patch::run(&self.workspace, &mut cache, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Create a directory and any necessary parent directories")]
    async fn create_directory(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<CreateDirectoryArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::create_directory::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Create or overwrite a file with UTF-8 content")]
    async fn create_file(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<CreateFileArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::create_file::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(
        description = "Get file metadata (size, mtime, type, existence) without reading contents"
    )]
    async fn read_file_info(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ReadFileInfoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::read_file_info::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Delete a file or directory recursively")]
    async fn delete_file(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<DeleteFileArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::delete_file::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(description = "Read entire file contents. maxBytes hard limit is 100KB")]
    async fn read_file(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ReadFileArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = ops::read_file::run(&self.workspace, params.0);
        Ok(to_tool_result(result))
    }

    #[tool(
        description = "Execute a shell command in a persistent isolated Docker container with the repo mounted at /workspace"
    )]
    async fn docker_shell(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<DockerShellArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = self.docker.run(params.0).await;
        Ok(to_tool_result(result))
    }
}

#[tool_handler]
impl ServerHandler for AgentMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "agent-mcp: high-performance file system operations for LLM agentic workflows. \
                Provides file listing, text search, code exploration, patch application, \
                and isolated Docker shell execution."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
