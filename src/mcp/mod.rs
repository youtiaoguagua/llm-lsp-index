//! MCP server module - exposes LSP capabilities as MCP tools

pub mod http_server;
pub mod protocol;
pub mod server;
pub mod tools;

pub use http_server::run_http_server;
pub use server::McpServer;
pub use tools::{McpTool, McpToolResponse, McpContent};