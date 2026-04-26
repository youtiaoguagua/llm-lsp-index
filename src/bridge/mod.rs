//! Bridge module - LSP to MCP mapping

pub mod handlers;
pub mod snippet;

pub use handlers::handle_tool_call;
pub use snippet::extract_snippet;