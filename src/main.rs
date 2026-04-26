//! LSP-to-MCP Bridge Entry Point
//!
//! Run as MCP Server over stdio for Claude Code integration.

use lsp_index::{Config, McpServer, LspRegistry};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("lsp_index=info".parse()?))
        .init();

    tracing::info!("LSP-to-MCP Bridge starting...");

    // Parse workspace from args or use current directory
    let workspace_root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    let config = Config::new(std::path::PathBuf::from(&workspace_root));

    // Detect language
    let registry = LspRegistry::new();
    if let Some(lsp_config) = registry.detect_language(&config.workspace_root) {
        tracing::info!("Detected language: {}", lsp_config.language);
    } else {
        tracing::warn!("No supported language detected in workspace");
    }

    // Start MCP Server
    let mut server = McpServer::new();
    server.set_workspace(config.workspace_root);
    server.run().await?;

    tracing::info!("LSP-to-MCP Bridge shutting down...");

    Ok(())
}