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
    match_data: RgSubmatchMatch,
}

#[derive(Debug, Deserialize)]
struct RgSubmatchMatch {
    start: usize,
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

    // Add globs
    for glob in &args.globs {
        cmd.arg("--glob").arg(glob);
    }

    // Pattern
    cmd.arg(&args.pattern);

    // Execute
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());

    let output = cmd.output().map_err(|e| {
        EngineError::new(ErrorCode::SearchFailed, "Failed to execute ripgrep")
            .with_details(json!({ "error": e.to_string() }))
    })?;

    if !output.status.success() && output.status.code() != Some(1) {
        // rg returns 1 if no matches, which is fine
        return Err(
            EngineError::new(ErrorCode::SearchFailed, "ripgrep command failed")
                .with_details(json!({ "code": output.status.code() })),
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
            let column = data
                .submatches
                .first()
                .map(|s| s.match_data.start as u32 + 1);

            matches.push(SearchMatch {
                file_path: data.path.text,
                line: data.line_number as u32,
                column,
                text: data.lines.text.trim().to_string(),
            });
        }
    }

    Ok(SearchTextResult { matches, truncated })
}

#[cfg(test)]
#[path = "search_text_test.rs"]
mod search_text_test;
