//! MCP server module - exposes LSP capabilities as MCP tools

mod server;
mod protocol;
pub mod tools;

pub use server::McpServer;
pub use tools::{McpTool, McpToolResponse, McpContent};