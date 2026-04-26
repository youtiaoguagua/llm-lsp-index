//! Global configuration for LSP-to-MCP Bridge

use std::path::PathBuf;

/// Configuration for the MCP server
pub struct Config {
    /// Workspace root directory
    pub workspace_root: PathBuf,
    /// Maximum lines in code snippets (default: 50)
    pub snippet_max_lines: usize,
    /// Maximum implementations to return (default: 3)
    pub max_implementations: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            snippet_max_lines: 50,
            max_implementations: 3,
        }
    }
}

impl Config {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            ..Default::default()
        }
    }
}