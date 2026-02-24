use crate::protocol::{DockerShellArgs, DockerShellResult, EngineError, ErrorCode};
use serde_json::json;
use std::sync::Mutex;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct DockerManager {
    container_id: Mutex<Option<String>>,
    repo_root: String,
    docker_image: String,
}

impl DockerManager {
    pub fn new(repo_root: String, docker_image: String) -> Self {
        Self {
            container_id: Mutex::new(None),
            repo_root,
            docker_image,
        }
    }

    async fn ensure_container(&self) -> Result<String, EngineError> {
        // Check if we already have a running container
        {
            let guard = self.container_id.lock().unwrap();
            if let Some(id) = guard.as_ref() {
                return Ok(id.clone());
            }
        }

        let container_name = format!("agent-mcp-{}", &format!("{:016x}", rand::random::<u64>()));

        let uid = get_uid();
        let gid = get_gid();

        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                &container_name,
                "--user",
                &format!("{}:{}", uid, gid),
                "-v",
                &format!("{}:/workspace", self.repo_root),
                "-w",
                "/workspace",
                &self.docker_image,
                "sleep",
                "infinity",
            ])
            .output()
            .await
            .map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to spawn docker run")
                    .with_details(json!({ "io": e.to_string() }))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(
                EngineError::new(ErrorCode::IoError, "Failed to start Docker container")
                    .with_details(json!({ "stderr": stderr.trim() })),
            );
        }

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        {
            let mut guard = self.container_id.lock().unwrap();
            *guard = Some(id.clone());
        }

        Ok(id)
    }

    pub async fn stop(&self) {
        let id = {
            let mut guard = self.container_id.lock().unwrap();
            guard.take()
        };
        if let Some(id) = id {
            let _ = Command::new("docker").args(["stop", &id]).output().await;
        }
    }

    pub async fn run(&self, args: DockerShellArgs) -> Result<DockerShellResult, EngineError> {
        if args.command.is_empty() {
            return Err(EngineError::new(
                ErrorCode::InvalidArgument,
                "command must be a non-empty string",
            ));
        }

        let container_id = self.ensure_container().await?;
        let work_dir = format!("/workspace/{}", args.work_dir.trim_matches('/'));

        let docker_args = [
            "exec",
            "-w",
            &work_dir,
            &container_id,
            "/bin/bash",
            "-c",
            &args.command,
        ];

        let duration = Duration::from_millis(args.timeout_ms);

        let result = timeout(duration, async {
            let mut child = Command::new("docker")
                .args(docker_args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    EngineError::new(ErrorCode::IoError, "Failed to spawn docker exec")
                        .with_details(json!({ "io": e.to_string() }))
                })?;

            // Read stdout and stderr concurrently using select to preserve ordering as
            // best we can with buffered reads. Collect into interleaved output vec.
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let mut output: Vec<(String, String)> = Vec::new();

            let mut stdout_buf = Vec::new();
            let mut stderr_buf = Vec::new();
            let mut stdout_done = false;
            let mut stderr_done = false;

            loop {
                if stdout_done && stderr_done {
                    break;
                }

                let mut so_chunk = vec![0u8; 4096];
                let mut se_chunk = vec![0u8; 4096];

                tokio::select! {
                    n = stdout.read(&mut so_chunk), if !stdout_done => {
                        match n {
                            Ok(0) => stdout_done = true,
                            Ok(n) => {
                                stdout_buf.extend_from_slice(&so_chunk[..n]);
                                if let Some(pos) = stdout_buf.iter().rposition(|&b| b == b'\n') {
                                    let chunk = String::from_utf8_lossy(&stdout_buf[..=pos]).into_owned();
                                    stdout_buf.drain(..=pos);
                                    output.push((chunk, "stdout".to_string()));
                                }
                            }
                            Err(_) => stdout_done = true,
                        }
                    }
                    n = stderr.read(&mut se_chunk), if !stderr_done => {
                        match n {
                            Ok(0) => stderr_done = true,
                            Ok(n) => {
                                stderr_buf.extend_from_slice(&se_chunk[..n]);
                                if let Some(pos) = stderr_buf.iter().rposition(|&b| b == b'\n') {
                                    let chunk = String::from_utf8_lossy(&stderr_buf[..=pos]).into_owned();
                                    stderr_buf.drain(..=pos);
                                    output.push((chunk, "stderr".to_string()));
                                }
                            }
                            Err(_) => stderr_done = true,
                        }
                    }
                }
            }

            // Flush any remaining buffered output
            if !stdout_buf.is_empty() {
                output.push((String::from_utf8_lossy(&stdout_buf).into_owned(), "stdout".to_string()));
            }
            if !stderr_buf.is_empty() {
                output.push((String::from_utf8_lossy(&stderr_buf).into_owned(), "stderr".to_string()));
            }

            let status = child.wait().await.map_err(|e| {
                EngineError::new(ErrorCode::IoError, "Failed to wait for docker exec")
                    .with_details(json!({ "io": e.to_string() }))
            })?;

            let exit_code = status.code().unwrap_or(-1);
            let success = status.success();

            Ok(DockerShellResult {
                exit_code,
                output,
                success,
            })
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_elapsed) => Err(EngineError::new(
                ErrorCode::IoError,
                format!("Command timed out after {}ms", args.timeout_ms),
            )),
        }
    }
}

impl Drop for DockerManager {
    fn drop(&mut self) {
        // Best-effort synchronous stop on drop. In practice the tokio runtime
        // will handle cleanup via the async stop() call from the signal handler.
        let id = self.container_id.lock().unwrap().take();
        if let Some(id) = id {
            let _ = std::process::Command::new("docker")
                .args(["stop", &id])
                .output();
        }
    }
}

#[cfg(unix)]
fn get_uid() -> u32 {
    unsafe { libc::getuid() }
}

#[cfg(not(unix))]
fn get_uid() -> u32 {
    1000
}

#[cfg(unix)]
fn get_gid() -> u32 {
    unsafe { libc::getgid() }
}

#[cfg(not(unix))]
fn get_gid() -> u32 {
    1000
}

#[cfg(test)]
#[path = "docker_shell_test.rs"]
mod docker_shell_test;
