use super::*;
use crate::protocol::DockerShellArgs;

fn make_manager() -> DockerManager {
    DockerManager::new(
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        "ubuntu:24.04".to_string(),
    )
}

/// Helper: skip the test if docker is not available on the host or not running in Linux mode.
async fn docker_available() -> bool {
    let output = Command::new("docker")
        .args(["info", "--format", "{{.OSType}}"])
        .output()
        .await;
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim() == "linux",
        _ => false,
    }
}

#[tokio::test]
async fn test_empty_command_is_rejected() {
    let mgr = make_manager();
    let args = DockerShellArgs {
        command: "".to_string(),
        timeout_ms: 5000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.code, ErrorCode::InvalidArgument));
}

#[tokio::test]
async fn test_simple_echo() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    let mgr = make_manager();
    let args = DockerShellArgs {
        command: "echo hello".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await.expect("echo should succeed");
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    let all_stdout: String = result
        .output
        .iter()
        .filter(|(_, stream)| stream == "stdout")
        .map(|(text, _)| text.as_str())
        .collect();
    assert!(
        all_stdout.contains("hello"),
        "expected 'hello' in stdout, got: {all_stdout:?}"
    );

    // Clean up
    mgr.stop().await;
}

#[tokio::test]
async fn test_exit_code_nonzero() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    let mgr = make_manager();
    let args = DockerShellArgs {
        command: "exit 42".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await.expect("command should complete");
    assert!(!result.success);
    assert_eq!(result.exit_code, 42);

    mgr.stop().await;
}

#[tokio::test]
async fn test_stderr_captured() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    let mgr = make_manager();
    let args = DockerShellArgs {
        command: "echo oops >&2".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await.expect("command should succeed");
    assert!(result.success);
    let all_stderr: String = result
        .output
        .iter()
        .filter(|(_, stream)| stream == "stderr")
        .map(|(text, _)| text.as_str())
        .collect();
    assert!(
        all_stderr.contains("oops"),
        "expected 'oops' in stderr, got: {all_stderr:?}"
    );

    mgr.stop().await;
}

#[tokio::test]
async fn test_container_is_reused_across_calls() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    let mgr = make_manager();

    // Write a file in the first call
    let args1 = DockerShellArgs {
        command: "echo persistent > /tmp/agent_mcp_test_marker".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    mgr.run(args1).await.expect("first call should succeed");

    // Read it back in a second call — same container, so the file is still there
    let args2 = DockerShellArgs {
        command: "cat /tmp/agent_mcp_test_marker".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args2).await.expect("second call should succeed");
    assert!(result.success);
    let all_stdout: String = result
        .output
        .iter()
        .filter(|(_, stream)| stream == "stdout")
        .map(|(text, _)| text.as_str())
        .collect();
    assert!(
        all_stdout.contains("persistent"),
        "expected container state to be preserved; got: {all_stdout:?}"
    );

    mgr.stop().await;
}

#[tokio::test]
async fn test_timeout_fires() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    let mgr = make_manager();
    let args = DockerShellArgs {
        command: "sleep 60".to_string(),
        timeout_ms: 500,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await;
    assert!(result.is_err(), "expected timeout error");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("timed out"),
        "expected 'timed out' in error message, got: {:?}",
        err.message
    );

    mgr.stop().await;
}

#[tokio::test]
async fn test_workspace_mounted() {
    if !docker_available().await {
        eprintln!("skipping: docker not available");
        return;
    }
    // Use the actual repo root so the mount is real
    let repo_root = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let mgr = DockerManager::new(repo_root, "ubuntu:24.04".to_string());

    let args = DockerShellArgs {
        command: "ls /workspace".to_string(),
        timeout_ms: 30_000,
        work_dir: ".".to_string(),
    };
    let result = mgr.run(args).await.expect("ls should succeed");
    assert!(result.success);
    let all_stdout: String = result
        .output
        .iter()
        .filter(|(_, stream)| stream == "stdout")
        .map(|(text, _)| text.as_str())
        .collect();
    // Cargo.toml should always be present in the workspace root
    assert!(
        all_stdout.contains("Cargo.toml"),
        "expected workspace contents to be visible; got: {all_stdout:?}"
    );

    mgr.stop().await;
}
