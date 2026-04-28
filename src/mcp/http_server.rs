//! MCP Server - StreamableHTTP mode
//!
//! HTTP-based MCP server supporting streamable responses.
//! Follows MCP 2024-11-05 protocol specification.

use axum::{
    extract::State,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::lsp::{LspClient, LspRegistry};
use crate::mcp::tools::McpTool;

/// MCP request from client
#[derive(Debug, Serialize, Deserialize)]
pub struct McpHttpRequest {
    pub jsonrpc: String,
    pub id: Option<i64>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// MCP response to client
#[derive(Debug, Serialize, Deserialize)]
pub struct McpHttpResponse {
    pub jsonrpc: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpHttpError>,
}

/// MCP error
#[derive(Debug, Serialize, Deserialize)]
pub struct McpHttpError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Shared state for HTTP server
pub struct HttpServerState {
    /// LSP client
    pub lsp_client: Arc<Mutex<Option<LspClient>>>,
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

impl HttpServerState {
    /// Create new server state
    pub async fn new(workspace_root: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = LspRegistry::new();
        let path = std::path::PathBuf::from(workspace_root);
        let config = registry.detect_language(&path);

        let lsp_client = if let Some(lsp_config) = config {
            tracing::info!("Initializing LSP for language: {}", lsp_config.language);

            let mut client = LspClient::new(lsp_config).await?;
            client.initialize(workspace_root).await?;

            tracing::info!("LSP client initialized successfully");
            Some(client)
        } else {
            tracing::warn!("No supported language detected, LSP not initialized");
            None
        };

        Ok(Self {
            lsp_client: Arc::new(Mutex::new(lsp_client)),
            name: "lsp-index".to_string(),
            version: "0.1.0".to_string(),
        })
    }
}

/// Handle MCP HTTP requests
async fn handle_mcp_request(
    State(state): State<Arc<HttpServerState>>,
    Json(request): Json<McpHttpRequest>,
) -> impl IntoResponse {
    tracing::info!("HTTP MCP request: {}", request.method);

    let id = request.id.unwrap_or(0);
    let method = request.method.clone();

    let response = match method.as_str() {
        "initialize" => handle_initialize(&state, request.id).await,
        "initialized" => handle_initialized(request.id),
        "tools/list" => handle_tools_list(&state, request.id).await,
        "tools/call" => handle_tools_call(&state, request, id).await,
        _ => create_error_response(
            id,
            -32601,
            format!("Method not found: {}", method),
        ),
    };

    Json(response)
}

/// Handle initialize request
async fn handle_initialize(state: &HttpServerState, id: Option<i64>) -> McpHttpResponse {
    McpHttpResponse {
        jsonrpc: "2.0".to_string(),
        id: id.unwrap_or(0),
        result: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": state.name,
                "version": state.version
            }
        })),
        error: None,
    }
}

/// Handle initialized notification
fn handle_initialized(id: Option<i64>) -> McpHttpResponse {
    McpHttpResponse {
        jsonrpc: "2.0".to_string(),
        id: id.unwrap_or(0),
        result: Some(serde_json::json!({})),
        error: None,
    }
}

/// Handle tools/list request
async fn handle_tools_list(_state: &HttpServerState, id: Option<i64>) -> McpHttpResponse {
    let tools = McpTool::definitions();

    McpHttpResponse {
        jsonrpc: "2.0".to_string(),
        id: id.unwrap_or(0),
        result: Some(serde_json::json!({ "tools": tools })),
        error: None,
    }
}

/// Handle tools/call request
async fn handle_tools_call(
    state: &HttpServerState,
    request: McpHttpRequest,
    id: i64,
) -> McpHttpResponse {
    let params = request.params.clone().unwrap_or(serde_json::json!({}));
    let tool_name = params.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let arguments = params.get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let mut client_guard = state.lsp_client.lock().await;

    match execute_tool(tool_name, &arguments, client_guard.as_mut()).await {
        Ok(result) => McpHttpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        },
        Err(e) => create_error_response(id, -1, e.to_string()),
    }
}

/// Execute a tool call
async fn execute_tool(
    tool_name: &str,
    arguments: &serde_json::Value,
    client: Option<&mut LspClient>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = client.ok_or("LSP client not initialized")?;

    match tool_name {
        "lsp_goto_definition" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;

            client.open_document(file_path).await?;

            let result = client.text_document_request(
                "textDocument/definition",
                file_path,
                line,
                character
            ).await?;

            parse_definition_result(result, false)
        }
        "lsp_find_references" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;

            client.open_document(file_path).await?;

            let result = client.find_references(file_path, line, character).await?;

            parse_references_result(result)
        }
        "lsp_hover" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let line = arguments["line"].as_u64().unwrap_or(0) as u32;
            let character = arguments["character"].as_u64().unwrap_or(0) as u32;

            client.open_document(file_path).await?;

            let result = client.text_document_request(
                "textDocument/hover",
                file_path,
                line,
                character
            ).await?;

            parse_hover_result(result)
        }
        "lsp_workspace_symbols" => {
            let query = arguments["query"].as_str().unwrap_or("");

            let result = client.workspace_symbols(query).await?;

            parse_symbols_result(result)
        }
        "lsp_get_diagnostics" => {
            let file_path = arguments["file_path"].as_str().unwrap_or("");
            let diagnostics = client.get_diagnostics(file_path);

            if diagnostics.is_empty() {
                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": "No diagnostics available for this file"
                    }]
                }))
            } else {
                let lines: Vec<String> = diagnostics
                    .iter()
                    .map(|d| {
                        let message = d.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown");
                        let severity = d.get("severity").and_then(|s| s.as_u64()).unwrap_or(1);
                        let severity_str = match severity {
                            1 => "Error",
                            2 => "Warning",
                            3 => "Info",
                            4 => "Hint",
                            _ => "Unknown",
                        };
                        format!("[{}] {}", severity_str, message)
                    })
                    .collect();

                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Found {} diagnostics:\n{}", lines.len(), lines.join("\n"))
                    }]
                }))
            }
        }
        _ => Err(format!("Unknown tool: {}", tool_name).into()),
    }
}

/// Parse definition result
fn parse_definition_result(
    result: serde_json::Value,
    _all_implementations: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use crate::bridge::snippet::extract_snippet;

    tracing::debug!("Parsing definition result: {:?}", result);

    let mut locations: Vec<(String, u32)> = Vec::new();

    // Handle single location
    if let Some(uri) = result.get("uri").and_then(|u| u.as_str()) {
        let path = crate::mcp::server::McpServer::uri_to_path(uri).unwrap_or(uri.to_string());
        let line = result.get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(|l| l.as_u64())
            .unwrap_or(0) as u32;
        locations.push((path, line));
    }

    // Handle LocationLink format
    if let Some(uri) = result.get("targetUri").and_then(|u| u.as_str()) {
        let path = crate::mcp::server::McpServer::uri_to_path(uri).unwrap_or(uri.to_string());
        let line = result.get("targetRange")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(|l| l.as_u64())
            .unwrap_or(0) as u32;
        locations.push((path, line));
    }

    // Handle array of locations
    if let Some(arr) = result.as_array() {
        for loc in arr {
            if let Some(uri) = loc.get("uri").and_then(|u| u.as_str()) {
                let path = crate::mcp::server::McpServer::uri_to_path(uri).unwrap_or(uri.to_string());
                let line = loc.get("range")
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as u32;
                locations.push((path, line));
            }
            if let Some(uri) = loc.get("targetUri").and_then(|u| u.as_str()) {
                let path = crate::mcp::server::McpServer::uri_to_path(uri).unwrap_or(uri.to_string());
                let line = loc.get("targetRange")
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as u32;
                locations.push((path, line));
            }
        }
    }

    let max = 3;
    locations = locations.into_iter().take(max).collect();

    if locations.is_empty() {
        return Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": "No definition found"
            }]
        }));
    }

    let mut snippets = Vec::new();
    for (path, line) in locations {
        let snippet = extract_snippet(&path, line, 20)?;
        snippets.push(format!("File: {}:{}\n{}", path, line + 1, snippet));
    }

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": snippets.join("\n\n---\n\n")
        }]
    }))
}

/// Parse references result
fn parse_references_result(result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut references: Vec<String> = Vec::new();

    if let Some(arr) = result.as_array() {
        for loc in arr {
            if let Some(uri) = loc.get("uri").and_then(|u| u.as_str()) {
                let path = crate::mcp::server::McpServer::uri_to_path(uri).unwrap_or(uri.to_string());
                let line = loc.get("range")
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as u32;
                references.push(format!("{}:{}", path, line + 1));
            }
        }
    }

    if references.is_empty() {
        return Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": "No references found"
            }]
        }));
    }

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": format!("Found {} references:\n{}", references.len(), references.join("\n"))
        }]
    }))
}

/// Parse hover result
fn parse_hover_result(result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let contents = result.get("contents");

    let text = if let Some(contents) = contents {
        // Handle MarkupContent
        if let Some(value) = contents.get("value").and_then(|v| v.as_str()) {
            value.to_string()
        }
        // Handle MarkedString (string or object)
        else if let Some(s) = contents.as_str() {
            s.to_string()
        }
        // Handle array of MarkedString
        else if let Some(arr) = contents.as_array() {
            arr.iter()
                .filter_map(|item| {
                    item.get("value").and_then(|v| v.as_str())
                        .or_else(|| item.as_str())
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        else {
            "Unable to parse hover contents".to_string()
        }
    } else {
        "No hover information available".to_string()
    };

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    }))
}

/// Parse symbols result
fn parse_symbols_result(result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut symbols: Vec<String> = Vec::new();

    if let Some(arr) = result.as_array() {
        for sym in arr {
            let name = sym.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            let kind = sym.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
            let kind_name = match kind {
                5 => "Class",
                9 => "Function",
                12 => "Variable",
                13 => "Constant",
                22 => "Struct",
                23 => "Enum",
                24 => "Interface",
                25 => "Method",
                _ => "Symbol"
            };

            let location = sym.get("location");
            let path = location
                .and_then(|l| l.get("uri"))
                .and_then(|u| u.as_str())
                .and_then(|u| crate::mcp::server::McpServer::uri_to_path(u))
                .unwrap_or_default();

            let line = location
                .and_then(|l| l.get("range"))
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(|l| l.as_u64())
                .unwrap_or(0);

            symbols.push(format!("{} ({}) @ {}:{}", name, kind_name, path, line + 1));
        }
    }

    if symbols.is_empty() {
        return Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": "No symbols found"
            }]
        }));
    }

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": format!("Found {} symbols:\n{}", symbols.len(), symbols.join("\n"))
        }]
    }))
}

/// Create error response
fn create_error_response(id: i64, code: i64, message: String) -> McpHttpResponse {
    McpHttpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(McpHttpError {
            code,
            message,
            data: None,
        }),
    }
}

/// Run HTTP MCP server
pub async fn run_http_server(
    workspace_root: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(HttpServerState::new(workspace_root).await?);

    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        .route("/mcp/v1", post(handle_mcp_request))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    tracing::info!("MCP HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
