use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl EngineError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    PathOutsideRoot,
    DriftDetected,
    SearchFailed,
    ParseFailed,
    IoError,
    #[allow(dead_code)]
    Internal,
}

// ---------- list_dir ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDirArgs {
    pub dir_path: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
}
fn default_depth() -> usize {
    4
}
fn default_max_entries() -> usize {
    2000
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDirResult {
    pub entries: Vec<ListDirEntry>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDirEntry {
    pub path: String,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

// ---------- search_text ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextArgs {
    pub pattern: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub globs: Vec<String>,
    #[serde(default)]
    pub case_sensitive: Option<bool>,
    #[serde(default)]
    pub regex: Option<bool>,
    #[serde(default = "default_no_ignore")]
    pub no_ignore: bool,
    #[serde(default = "default_max_matches")]
    pub max_matches: usize,
    #[serde(default = "default_max_per_file")]
    pub max_per_file: usize,
    #[serde(default = "default_max_line_length")]
    pub max_line_length: usize,
}
fn default_no_ignore() -> bool {
    true
}
fn default_max_matches() -> usize {
    200
}
fn default_max_per_file() -> usize {
    50
}
fn default_max_line_length() -> usize {
    200
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub file_path: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    pub text: String,
}

// ---------- read_excerpt ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadExcerptArgs {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    /// Maximum bytes to return. Hard limit of 100KB (102,400 bytes) is enforced.
    /// Values above 100KB will be capped to 100KB. Default is 20KB.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}
fn default_max_bytes() -> usize {
    20_000
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadExcerptResult {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub text: String,
    pub truncated: bool,
}

// ---------- explore_code ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreCodeArgs {
    pub file_path: String,
    pub exported: bool,
    #[serde(default = "default_max_symbols")]
    pub max_symbols: usize,
}
fn default_max_symbols() -> usize {
    200
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreCodeResult {
    pub file_path: String,
    pub language: String,
    pub symbols: Vec<ExploredSymbol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploredSymbol {
    pub name: String,
    pub kind: String,
    pub exported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub location: SymbolLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolLocation {
    pub start_line: u32,
    pub end_line: u32,
}

// ---------- apply_patch ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPatchArgs {
    pub file_path: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    pub edits: Vec<PatchEdit>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PatchEdit {
    Insert {
        #[serde(rename = "where")]
        where_: String,
        anchor: PatchAnchor,
        text: String,
    },
    Replace {
        target: PatchRegion,
        replacement: String,
    },
    Delete {
        target: PatchRegion,
    },
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchAnchor {
    pub needle: String,
    #[serde(default)]
    pub require_unique: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchRegion {
    pub start: PatchAnchor,
    #[serde(default)]
    pub end: Option<PatchAnchor>,
    #[serde(default)]
    pub include_end_needle: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPatchResult {
    pub file_path: String,
    pub dry_run: bool,
    pub applied: Vec<AppliedEditSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedEditSummary {
    pub kind: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

// ---------- create_directory ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectoryArgs {
    pub dir_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectoryResult {
    pub dir_path: String,
    pub created: bool,
}

// ---------- create_file ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileArgs {
    pub file_path: String,
    pub contents: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}
fn default_encoding() -> String {
    "utf8".to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileResult {
    pub file_path: String,
    pub created: bool,
    pub bytes_written: usize,
}

// ---------- read_file_info ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileInfoArgs {
    pub file_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileInfoResult {
    pub file_path: String,
    pub exists: bool,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_ns: Option<i128>,
}

// ---------- delete_file ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFileArgs {
    pub file_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFileResult {
    pub file_path: String,
    pub deleted: bool,
}

// ---------- read_file ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileArgs {
    pub file_path: String,
    /// Maximum bytes to read. Hard limit of 100KB (102,400 bytes) is enforced.
    #[serde(default = "default_max_file_bytes")]
    pub max_bytes: usize,
}
fn default_max_file_bytes() -> usize {
    102_400
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileResult {
    pub file_path: String,
    pub contents: String,
    pub size: u64,
    pub truncated: bool,
}

// ---------- docker_shell ----------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DockerShellArgs {
    /// Shell command to execute inside the container.
    pub command: String,
    /// Timeout in milliseconds (default: 30000).
    #[serde(default = "default_docker_timeout_ms")]
    pub timeout_ms: u64,
    /// Working directory relative to /workspace (default: ".").
    #[serde(default = "default_work_dir")]
    pub work_dir: String,
}
fn default_docker_timeout_ms() -> u64 {
    30_000
}
fn default_work_dir() -> String {
    ".".to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerShellResult {
    pub exit_code: i32,
    pub output: Vec<(String, String)>,
    pub success: bool,
}
