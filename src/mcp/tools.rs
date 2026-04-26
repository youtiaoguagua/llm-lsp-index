//! MCP Tools - LSP capabilities exposed as MCP tools

use serde::{Deserialize, Serialize};

/// MCP Tool input parameters
#[derive(Debug, Deserialize)]
pub struct McpToolInput {
    /// Tool name
    pub name: String,
    /// Tool arguments
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// MCP Tool response
#[derive(Debug, Serialize)]
pub struct McpToolResponse {
    pub content: Vec<McpContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP Content block
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
}

/// Available MCP tools
#[derive(Debug, Deserialize)]
pub enum McpTool {
    /// Go to definition
    GotoDefinition {
        file_path: String,
        line: u32,
        character: u32,
        #[serde(default)]
        all_implementations: bool,
    },
    /// Find references
    FindReferences {
        file_path: String,
        line: u32,
        character: u32,
    },
    /// Hover (type info + docstring)
    Hover {
        file_path: String,
        line: u32,
        character: u32,
    },
    /// Workspace symbols search
    WorkspaceSymbols {
        query: String,
    },
    /// Get diagnostics
    GetDiagnostics {
        file_path: String,
    },
}

impl McpTool {
    /// Get tool definitions for MCP tools/list
    pub fn definitions() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "name": "lsp_goto_definition",
                "description": "Go to definition of symbol at position. Returns target file and code snippet.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "Absolute file path"},
                        "line": {"type": "integer", "description": "Line number (0-based)"},
                        "character": {"type": "integer", "description": "Character offset (0-based)"},
                        "all_implementations": {"type": "boolean", "description": "Return all implementations (default: top 3)", "default": false}
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            serde_json::json!({
                "name": "lsp_find_references",
                "description": "Find all references to symbol at position across the workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "Absolute file path"},
                        "line": {"type": "integer", "description": "Line number (0-based)"},
                        "character": {"type": "integer", "description": "Character offset (0-based)"}
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            serde_json::json!({
                "name": "lsp_hover",
                "description": "Get type signature and documentation for symbol at position.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "Absolute file path"},
                        "line": {"type": "integer", "description": "Line number (0-based)"},
                        "character": {"type": "integer", "description": "Character offset (0-based)"}
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            serde_json::json!({
                "name": "lsp_workspace_symbols",
                "description": "Search for symbols (classes, functions, etc.) by name across the workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Symbol name or partial match"}
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "lsp_get_diagnostics",
                "description": "Get lint and type errors for a file.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "Absolute file path"}
                    },
                    "required": ["file_path"]
                }
            }),
            serde_json::json!({
                "name": "lsp_hybrid_search",
                "description": "Search for symbols and text matches across the workspace. Combines LSP symbol search with ripgrep-like text search to find TODOs, comments, strings, and symbols.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query (symbol name or text pattern)"},
                        "include_symbols": {"type": "boolean", "description": "Include LSP symbol results (default: true)", "default": true},
                        "include_text": {"type": "boolean", "description": "Include text search results (default: true)", "default": true},
                        "file_types": {"type": "array", "description": "Filter by file extensions (e.g., [\"rs\", \"toml\"])", "items": {"type": "string"}},
                        "max_results": {"type": "integer", "description": "Maximum total results (default: 10)", "default": 10}
                    },
                    "required": ["query"]
                }
            }),
        ]
    }
}