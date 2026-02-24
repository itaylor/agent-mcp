use crate::cache::RopeCache;
use crate::protocol::{EngineError, ErrorCode, ReadExcerptArgs, ReadExcerptResult};
use crate::workspace::Workspace;
use ropey::RopeSlice;
use serde_json::json;

const MAX_EXCERPT_BYTES: usize = 102_400; // 100KB hard limit

pub fn run(
    ws: &Workspace,
    cache: &mut RopeCache,
    args: ReadExcerptArgs,
) -> Result<ReadExcerptResult, EngineError> {
    // Enforce hard limit - cap user's max_bytes to our maximum
    let max_bytes = args.max_bytes.min(MAX_EXCERPT_BYTES);

    if args.start_line == 0 || args.end_line == 0 || args.end_line < args.start_line {
        return Err(
            EngineError::new(ErrorCode::InvalidArgument, "Invalid line range").with_details(
                json!({
                    "startLine": args.start_line,
                    "endLine": args.end_line
                }),
            ),
        );
    }

    let (_resolved, rope) = cache.get_fresh_rope(ws, &args.file_path)?;

    // Ropey lines are 0-based internally
    let total_lines = rope.len_lines() as u32; // includes final line even if no newline
    if args.start_line > total_lines {
        return Err(
            EngineError::new(ErrorCode::InvalidArgument, "startLine beyond end of file")
                .with_details(json!({
                    "startLine": args.start_line,
                    "totalLines": total_lines
                })),
        );
    }

    let end_line = args.end_line.min(total_lines);

    let start0 = args.start_line - 1; // to 0-based
                                      // For slicing inclusive endLine, we slice up to the start of (endLine) as 0-based end index + 1
                                      // Ropey line_to_char(i) gives char index at start of line i.
    let start_char = rope.line_to_char(start0 as usize);
    let end_char = rope.line_to_char(end_line as usize); // exclusive char index at start of line (end_line)

    let slice: RopeSlice = rope.slice(start_char..end_char);
    let full_text = slice.to_string();

    let (text, truncated) = truncate_to_max_bytes(&full_text, max_bytes);

    Ok(ReadExcerptResult {
        file_path: args.file_path,
        start_line: args.start_line,
        end_line,
        text,
        truncated,
    })
}

/// Truncate a UTF-8 string to max_bytes without cutting a codepoint in half.
fn truncate_to_max_bytes(s: &str, max_bytes: usize) -> (String, bool) {
    if s.len() <= max_bytes {
        return (s.to_string(), false);
    }
    if max_bytes == 0 {
        return ("".to_string(), true);
    }

    // Walk char boundaries until we'd exceed max_bytes
    let mut out = String::with_capacity(max_bytes);
    let mut used = 0usize;
    for ch in s.chars() {
        let mut buf = [0u8; 4];
        let enc = ch.encode_utf8(&mut buf);
        let clen = enc.len();
        if used + clen > max_bytes {
            break;
        }
        out.push(ch);
        used += clen;
    }
    (out, true)
}

#[cfg(test)]
#[path = "read_excerpt_test.rs"]
mod read_excerpt_test;
