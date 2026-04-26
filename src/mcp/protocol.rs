//! MCP Protocol types and utilities

use serde::{Deserialize, Serialize};

/// MCP Request
#[derive(Debug, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<i64>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// MCP Response
#[derive(Debug, Serialize, Deserialize)]
pub struct McpResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: &'static str,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP Error
#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP Tool definition
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}