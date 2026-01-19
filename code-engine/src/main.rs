mod cache;
mod protocol;
mod workspace;

mod ops {
    pub mod apply_patch;
    pub mod create_directory;
    pub mod create_file;
    pub mod delete_file;
    pub mod explore_code;
    pub mod list_dir;
    pub mod read_excerpt;
    pub mod read_file;
    pub mod read_file_info;
    pub mod search_text;
}

use protocol::*;
use std::io::{self, BufRead, Write};

struct EngineState {
    ws: workspace::Workspace,
    cache: cache::RopeCache,
}

fn main() -> anyhow::Result<()> {
    let repo_root = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let ws = workspace::Workspace::new(repo_root)?;
    let mut state = EngineState {
        ws,
        cache: cache::RopeCache::new(),
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: EngineRequest = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = EngineResponse::err(
                    "unknown".to_string(),
                    EngineError::new(ErrorCode::InvalidArgument, format!("Bad JSON: {e}")),
                );
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let resp = handle_request(&mut state, req);
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(state: &mut EngineState, req: EngineRequest) -> EngineResponse {
    let id = req.id.clone();

    let result = match req.op {
        OpName::ListDir => {
            let args: ListDirArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::list_dir::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::SearchText => {
            let args: SearchTextArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::search_text::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::ReadExcerpt => {
            let args: ReadExcerptArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::read_excerpt::run(&state.ws, &mut state.cache, args)
                .map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::ExploreCode => {
            let args: ExploreCodeArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::explore_code::run(&state.ws, &mut state.cache, args)
                .map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::ApplyPatch => {
            let args: ApplyPatchArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::apply_patch::run(&state.ws, &mut state.cache, args)
                .map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::CreateDirectory => {
            let args: CreateDirectoryArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::create_directory::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::CreateFile => {
            let args: CreateFileArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::create_file::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::ReadFileInfo => {
            let args: ReadFileInfoArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::read_file_info::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::DeleteFile => {
            let args: DeleteFileArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::delete_file::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
        OpName::ReadFile => {
            let args: ReadFileArgs = match parse_args(req.args) {
                Ok(a) => a,
                Err(e) => return EngineResponse::err(id, e),
            };
            ops::read_file::run(&state.ws, args).map(|r| serde_json::to_value(r).unwrap())
        }
    };

    match result {
        Ok(val) => EngineResponse::ok(id, val),
        Err(err) => EngineResponse::err(id, err),
    }
}

fn parse_args<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> Result<T, EngineError> {
    serde_json::from_value(v).map_err(|e| {
        EngineError::new(ErrorCode::InvalidArgument, "Invalid arguments")
            .with_details(serde_json::json!({ "serde": e.to_string() }))
    })
}
