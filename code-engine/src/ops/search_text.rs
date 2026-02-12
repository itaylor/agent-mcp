use crate::protocol::{EngineError, ErrorCode, SearchMatch, SearchTextArgs, SearchTextResult};
use crate::workspace::Workspace;
use serde::Deserialize;
use serde_json::json;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum RgMessage {
    #[serde(rename = "match")]
    Match { data: RgMatchData },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct RgMatchData {
    path: RgPath,
    lines: RgLines,
    line_number: u64,
    #[serde(default)]
    submatches: Vec<RgSubmatch>,
}

#[derive(Debug, Deserialize)]
struct RgPath {
    text: String,
}

#[derive(Debug, Deserialize)]
struct RgLines {
    text: String,
}

#[derive(Debug, Deserialize)]
struct RgSubmatch {
    #[serde(rename = "match")]
    #[allow(dead_code)]
    match_data: RgMatch,
    start: usize,
    #[allow(dead_code)]
    end: usize,
}

#[derive(Debug, Deserialize)]
struct RgMatch {
    #[allow(dead_code)]
    text: String,
}

pub fn run(ws: &Workspace, args: SearchTextArgs) -> Result<SearchTextResult, EngineError> {
    let cwd = args.cwd.as_deref().unwrap_or(".");
    let search_dir = ws.resolve_path(cwd)?;

    if !search_dir.exists() {
        return Err(
            EngineError::new(ErrorCode::NotFound, "Search directory not found")
                .with_details(json!({ "cwd": cwd })),
        );
    }

    // Build ripgrep command
    let mut cmd = Command::new("rg");
    cmd.arg("--json")
        .arg("--no-messages")
        .arg("--max-count")
        .arg(args.max_per_file.to_string())
        .current_dir(&search_dir);

    // Case sensitivity
    if args.case_sensitive == Some(true) {
        cmd.arg("--case-sensitive");
    } else {
        cmd.arg("--ignore-case");
    }

    // Regex mode (default is true for ripgrep)
    if args.regex == Some(false) {
        cmd.arg("--fixed-strings");
    }

    // Ignore files (default is to ignore .gitignore, etc.)
    if args.no_ignore {
        cmd.arg("--no-ignore");
    }

    // Add globs
    for glob in &args.globs {
        cmd.arg("--glob").arg(glob);
    }

    // Pattern
    cmd.arg(&args.pattern);

    // Search path (. means current directory)
    cmd.arg(".");

    // Execute
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| {
        EngineError::new(ErrorCode::SearchFailed, "Failed to execute ripgrep")
            .with_details(json!({ "error": e.to_string() }))
    })?;

    if !output.status.success() && output.status.code() != Some(1) {
        // rg returns 1 if no matches, which is fine
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(
            EngineError::new(ErrorCode::SearchFailed, "ripgrep command failed").with_details(
                json!({
                    "code": output.status.code(),
                    "stderr": stderr,
                    "stdout": stdout
                }),
            ),
        );
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches = Vec::new();
    let mut truncated = false;

    for line in stdout.lines() {
        if matches.len() >= args.max_matches {
            truncated = true;
            break;
        }

        let msg: RgMessage = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if let RgMessage::Match { data } = msg {
            let column = data.submatches.first().map(|s| s.start as u32 + 1);
            let line_text = data.lines.text.trim();

            // Truncate long lines intelligently (ripgrep's --max-columns doesn't work with --json)
            let text = if line_text.len() > args.max_line_length {
                truncate_around_match(line_text, &data.submatches, args.max_line_length)
            } else {
                line_text.to_string()
            };

            matches.push(SearchMatch {
                file_path: data.path.text,
                line: data.line_number as u32,
                column,
                text,
            });
        }
    }

    Ok(SearchTextResult { matches, truncated })
}

/// Truncates a long line, centering around the first match with indicators showing truncated chars.
/// Ripgrep's --max-columns flag doesn't work with --json output, so we handle truncation ourselves.
fn truncate_around_match(line: &str, submatches: &[RgSubmatch], max_len: usize) -> String {
    // If no submatches, truncate from start
    let match_start = submatches.first().map(|s| s.start).unwrap_or(0);

    // Reserve space for indicators like "[+1234 chars]"
    let indicator_space = 20; // rough estimate for " [+NNNN chars]" on each side
    let content_space = max_len.saturating_sub(indicator_space * 2);

    if content_space < 20 {
        // If max_len is too small, just truncate from start
        return format!("{}...", &line[..max_len.min(line.len())]);
    }

    // Try to center around the match
    let half_content = content_space / 2;
    let start = match_start.saturating_sub(half_content);
    let end = (start + content_space).min(line.len());

    // Adjust start if we hit the end
    let start = if end == line.len() && end > content_space {
        end - content_space
    } else {
        start
    };

    let snippet = &line[start..end];

    let prefix = if start > 0 {
        format!("[+{} chars] ", start)
    } else {
        String::new()
    };

    let suffix = if end < line.len() {
        format!(" [+{} chars]", line.len() - end)
    } else {
        String::new()
    };

    format!("{}{}{}", prefix, snippet, suffix)
}

#[cfg(test)]
#[path = "search_text_test.rs"]
mod search_text_test;
