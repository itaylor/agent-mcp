mod cache;
mod ops;
mod protocol;
mod server;
mod workspace;

use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let repo_root = std::env::var("REPO_ROOT")
        .ok()
        .or_else(|| std::env::args().nth(1))
        .unwrap_or_else(|| ".".to_string());

    eprintln!("agent-mcp starting, repo_root={repo_root}");

    let server = server::AgentMcp::new(&repo_root)?;

    // Graceful shutdown: stop the docker container when we get a signal.
    let server_clone = server.clone();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::spawn(async move {
            tokio::select! {
                _ = sigint.recv() => {}
                _ = sigterm.recv() => {}
            }
            server_clone.shutdown().await;
            std::process::exit(0);
        });
    }

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
