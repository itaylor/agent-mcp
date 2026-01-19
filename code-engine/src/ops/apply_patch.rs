use crate::cache::RopeCache;
use crate::protocol::{
    AppliedEditSummary, ApplyPatchArgs, ApplyPatchResult, EngineError, ErrorCode, PatchEdit,
};
use crate::workspace::Workspace;
use ropey::Rope;
use serde_json::json;
use std::fs;
use std::io::Write;

pub fn run(
    ws: &Workspace,
    cache: &mut RopeCache,
    args: ApplyPatchArgs,
) -> Result<ApplyPatchResult, EngineError> {
    let (_resolved, rope) = cache.get_fresh_rope(ws, &args.file_path)?;

    // Convert rope to string for anchor matching
    let content = rope.to_string();

    let mode = args.mode.as_deref().unwrap_or("strict");
    let strict = mode == "strict";

    // Plan all edits
    let mut planned_edits = Vec::new();

    for (idx, edit) in args.edits.iter().enumerate() {
        match plan_edit(edit, &content, strict, idx)? {
            Some(planned) => planned_edits.push(planned),
            None => continue,
        }
    }

    // Sort edits by start position in reverse order (so we can apply back-to-front)
    planned_edits.sort_by_key(|e| std::cmp::Reverse(e.start_byte));

    // Check for overlaps in strict mode
    if strict {
        for i in 0..planned_edits.len() {
            for j in (i + 1)..planned_edits.len() {
                let a = &planned_edits[i];
                let b = &planned_edits[j];
                if ranges_overlap(a.start_byte, a.end_byte, b.start_byte, b.end_byte) {
                    return Err(EngineError::new(ErrorCode::DriftDetected, "Edits overlap")
                        .with_details(json!({
                            "edit1": i,
                            "edit2": j
                        })));
                }
            }
        }
    }

    // Apply edits to a new rope
    let mut new_rope = rope.clone();

    for planned in &planned_edits {
        let start_char = byte_to_char_idx(&new_rope, planned.start_byte);
        let end_char = byte_to_char_idx(&new_rope, planned.end_byte);

        // Remove the range
        new_rope.remove(start_char..end_char);

        // Insert replacement text
        if !planned.replacement.is_empty() {
            new_rope.insert(start_char, &planned.replacement);
        }
    }

    // Build summaries
    let mut applied = Vec::new();
    for planned in &planned_edits {
        let start_line = byte_to_line(&content, planned.start_byte);
        let end_line = byte_to_line(&content, planned.end_byte);

        applied.push(AppliedEditSummary {
            kind: planned.kind.clone(),
            summary: planned.summary.clone(),
            start_line: Some(start_line),
            end_line: Some(end_line),
        });
    }

    // Write file if not dry-run
    if !args.dry_run {
        write_file_atomic(ws, &args.file_path, &new_rope)?;
        cache.put_rope(ws, &args.file_path, new_rope)?;
    }

    Ok(ApplyPatchResult {
        file_path: args.file_path,
        dry_run: args.dry_run,
        applied,
    })
}

struct PlannedEdit {
    start_byte: usize,
    end_byte: usize,
    replacement: String,
    kind: String,
    summary: String,
}

fn plan_edit(
    edit: &PatchEdit,
    content: &str,
    strict: bool,
    idx: usize,
) -> Result<Option<PlannedEdit>, EngineError> {
    match edit {
        PatchEdit::Insert {
            where_,
            anchor,
            text,
        } => {
            let positions = find_needle(content, &anchor.needle);

            if positions.is_empty() {
                if strict {
                    return Err(
                        EngineError::new(ErrorCode::DriftDetected, "Anchor not found")
                            .with_details(json!({
                                "edit": idx,
                                "anchor": anchor.needle
                            })),
                    );
                }
                return Ok(None);
            }

            if anchor.require_unique.unwrap_or(false) && positions.len() > 1 {
                return Err(
                    EngineError::new(ErrorCode::DriftDetected, "Anchor is not unique")
                        .with_details(json!({
                            "edit": idx,
                            "anchor": anchor.needle,
                            "count": positions.len()
                        })),
                );
            }

            let anchor_pos = positions[0];
            let (start, end) = match where_.as_str() {
                "before" => (anchor_pos, anchor_pos),
                "after" => {
                    let after_pos = anchor_pos + anchor.needle.len();
                    (after_pos, after_pos)
                }
                _ => {
                    return Err(EngineError::new(
                        ErrorCode::InvalidArgument,
                        "Invalid 'where' value",
                    )
                    .with_details(json!({ "where": where_ })))
                }
            };

            Ok(Some(PlannedEdit {
                start_byte: start,
                end_byte: end,
                replacement: text.clone(),
                kind: "insert".to_string(),
                summary: format!("Insert {} bytes {} anchor", text.len(), where_),
            }))
        }

        PatchEdit::Replace {
            target,
            replacement,
        } => {
            let (start, end) = find_region(content, target, strict, idx)?;

            Ok(Some(PlannedEdit {
                start_byte: start,
                end_byte: end,
                replacement: replacement.clone(),
                kind: "replace".to_string(),
                summary: format!(
                    "Replace {} bytes with {} bytes",
                    end - start,
                    replacement.len()
                ),
            }))
        }

        PatchEdit::Delete { target } => {
            let (start, end) = find_region(content, target, strict, idx)?;

            Ok(Some(PlannedEdit {
                start_byte: start,
                end_byte: end,
                replacement: String::new(),
                kind: "delete".to_string(),
                summary: format!("Delete {} bytes", end - start),
            }))
        }
    }
}

fn find_region(
    content: &str,
    target: &crate::protocol::PatchRegion,
    strict: bool,
    idx: usize,
) -> Result<(usize, usize), EngineError> {
    let start_positions = find_needle(content, &target.start.needle);

    if start_positions.is_empty() {
        if strict {
            return Err(
                EngineError::new(ErrorCode::DriftDetected, "Start anchor not found").with_details(
                    json!({
                        "edit": idx,
                        "anchor": target.start.needle
                    }),
                ),
            );
        }
        return Err(EngineError::new(
            ErrorCode::DriftDetected,
            "Start anchor not found",
        ));
    }

    if target.start.require_unique.unwrap_or(false) && start_positions.len() > 1 {
        return Err(
            EngineError::new(ErrorCode::DriftDetected, "Start anchor is not unique").with_details(
                json!({
                    "edit": idx,
                    "anchor": target.start.needle,
                    "count": start_positions.len()
                }),
            ),
        );
    }

    let start_pos = start_positions[0];
    let start_byte = start_pos + target.start.needle.len(); // Start after the needle

    let end_byte = if let Some(end_anchor) = &target.end {
        let end_positions = find_needle(&content[start_byte..], &end_anchor.needle);

        if end_positions.is_empty() {
            if strict {
                return Err(
                    EngineError::new(ErrorCode::DriftDetected, "End anchor not found")
                        .with_details(json!({
                            "edit": idx,
                            "anchor": end_anchor.needle
                        })),
                );
            }
            return Err(EngineError::new(
                ErrorCode::DriftDetected,
                "End anchor not found",
            ));
        }

        if end_anchor.require_unique.unwrap_or(false) && end_positions.len() > 1 {
            return Err(
                EngineError::new(ErrorCode::DriftDetected, "End anchor is not unique")
                    .with_details(json!({
                        "edit": idx,
                        "anchor": end_anchor.needle,
                        "count": end_positions.len()
                    })),
            );
        }

        let end_pos = start_byte + end_positions[0];
        if target.include_end_needle.unwrap_or(false) {
            end_pos + end_anchor.needle.len()
        } else {
            end_pos
        }
    } else {
        start_byte
    };

    Ok((start_pos, end_byte))
}

fn find_needle(haystack: &str, needle: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut start = 0;

    while let Some(pos) = haystack[start..].find(needle) {
        let absolute_pos = start + pos;
        positions.push(absolute_pos);
        start = absolute_pos + 1;
    }

    positions
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

fn byte_to_char_idx(rope: &Rope, byte_idx: usize) -> usize {
    let mut char_idx = 0;
    let mut bytes = 0;

    for chunk in rope.chunks() {
        let chunk_bytes = chunk.len();
        if bytes + chunk_bytes > byte_idx {
            // The byte is in this chunk
            let offset = byte_idx - bytes;
            let mut byte_pos = 0;
            for ch in chunk.chars() {
                if byte_pos >= offset {
                    return char_idx;
                }
                byte_pos += ch.len_utf8();
                char_idx += 1;
            }
            return char_idx;
        }
        bytes += chunk_bytes;
        char_idx += chunk.chars().count();
    }

    char_idx
}

fn byte_to_line(content: &str, byte_idx: usize) -> u32 {
    let prefix = &content[..byte_idx.min(content.len())];
    prefix.lines().count() as u32
}

fn write_file_atomic(ws: &Workspace, user_path: &str, rope: &Rope) -> Result<(), EngineError> {
    let resolved = ws.resolve_path(user_path)?;

    // Write to a temporary file first
    let tmp_path = resolved.with_extension("tmp");

    {
        let mut file = fs::File::create(&tmp_path).map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to create temporary file")
                .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;

        for chunk in rope.chunks() {
            file.write_all(chunk.as_bytes()).map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to write to temporary file")
                    .with_details(json!({ "path": user_path, "io": e.to_string() }))
            })?;
        }

        file.sync_all().map_err(|e| {
            EngineError::new(ErrorCode::IoError, "Failed to sync temporary file")
                .with_details(json!({ "path": user_path, "io": e.to_string() }))
        })?;
    }

    // Atomic rename
    fs::rename(&tmp_path, &resolved).map_err(|e| {
        // Clean up temp file on error
        let _ = fs::remove_file(&tmp_path);
        EngineError::new(ErrorCode::IoError, "Failed to rename temporary file")
            .with_details(json!({ "path": user_path, "io": e.to_string() }))
    })?;

    Ok(())
}

#[cfg(test)]
#[path = "apply_patch_test.rs"]
mod apply_patch_test;
