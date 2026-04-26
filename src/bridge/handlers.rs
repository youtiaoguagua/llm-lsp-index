//! Tool call handlers - map MCP tool calls to LSP requests

use crate::mcp::tools::{McpTool, McpToolResponse, McpContent};

/// Handle an MCP tool call
pub async fn handle_tool_call(
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<McpToolResponse, Box<dyn std::error::Error>> {
    tracing::info!("Handling tool call: {}", tool_name);

    match tool_name {
        "lsp_goto_definition" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;
            let all_implementations = arguments["all_implementations"].as_bool().unwrap_or(false);

            // TODO: Call actual LSP
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!(
                        "goto_definition: {}:{}:{} (all={})",
                        file_path, line, character, all_implementations
                    ),
                }],
                is_error: None,
            })
        }
        "lsp_find_references" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;

            // TODO: Call actual LSP
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!("find_references: {}:{}:{}", file_path, line, character),
                }],
                is_error: None,
            })
        }
        "lsp_hover" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;

            // TODO: Call actual LSP
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!("hover: {}:{}:{}", file_path, line, character),
                }],
                is_error: None,
            })
        }
        "lsp_workspace_symbols" => {
            let query = arguments["query"].as_str().unwrap_or("");

            // TODO: Call actual LSP
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!("workspace_symbols: {}", query),
                }],
                is_error: None,
            })
        }
        "lsp_get_diagnostics" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");

            // TODO: Call actual LSP
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!("diagnostics: {}", file_path),
                }],
                is_error: None,
            })
        }
        _ => {
            Ok(McpToolResponse {
                content: vec![McpContent::Text {
                    text: format!("Unknown tool: {}", tool_name),
                }],
                is_error: Some(true),
            })
        }
    }
}