//! LSP-to-MCP Bridge HTTP Server Entry Point
//!
//! Run as MCP Server over HTTP for remote/REST API access.

use lsp_index::mcp::run_http_server;
use tracing_subscriber::EnvFilter;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("lsp_index=info".parse()?))
        .init();

    tracing::info!("LSP-to-MCP Bridge HTTP Server starting...");

    // Parse workspace from args or use current directory
    let workspace_root = env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    // Parse port from args or use default
    let port = env::args()
        .nth(2)
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000u16);

    tracing::info!("Workspace: {}", workspace_root);
    tracing::info!("Port: {}", port);

    // Run HTTP server
    run_http_server(&workspace_root, port).await?;

    tracing::info!("LSP-to-MCP Bridge HTTP Server shutting down...");

    Ok(())
}
