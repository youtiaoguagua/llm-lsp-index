//! LSP-to-MCP Bridge
//!
//! A headless LSP client that exposes IDE capabilities (goto definition, find references, hover)
//! to AI models via MCP (Model Context Protocol).

pub mod config;
pub mod lsp;
pub mod mcp;
pub mod bridge;
pub mod utils;
pub mod search;

pub use config::Config;
pub use lsp::{LspClient, LspRegistry};
pub use mcp::McpServer;
pub use search::{search_text, TextMatch, SearchOptions};