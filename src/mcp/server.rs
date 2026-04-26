//! MCP Server - stdio-based MCP protocol implementation

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use crate::lsp::watcher::{FileWatcher, FileChangeEvent};
use crate::mcp::protocol::{McpRequest, McpResponse, McpError};
use crate::mcp::tools::McpTool;
use crate::bridge::handlers::handle_tool_call;
use crate::lsp::{LspClient, LspRegistry};
use tokio::sync::mpsc;

/// MCP Server running in stdio mode
pub struct McpServer {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// LSP client (if initialized)
    lsp_client: Option<LspClient>,
    /// Workspace root
    workspace_root: PathBuf,
    /// File watcher (if initialized)
    file_watcher: Option<FileWatcher>,
    /// Channel for file change events
    change_receiver: Option<mpsc::UnboundedReceiver<FileChangeEvent>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self {
            name: "lsp-index".to_string(),
            version: "0.1.0".to_string(),
            lsp_client: None,
            workspace_root: PathBuf::from("."),
            file_watcher: None,
            change_receiver: None,
        }
    }

    /// Set workspace root
    pub fn set_workspace(&mut self, workspace: PathBuf) {
        self.workspace_root = workspace;
    }

    /// Initialize LSP client for the workspace
    pub async fn init_lsp(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let registry = LspRegistry::new();
        tracing::info!("Detecting language in workspace: {:?}", self.workspace_root);
        let config = registry.detect_language(&self.workspace_root);

        if let Some(lsp_config) = config {
            tracing::info!("Initializing LSP for language: {}", lsp_config.language);

            let mut client = LspClient::new(lsp_config).await?;
            client.initialize(&self.workspace_root.to_string_lossy()).await?;

            self.lsp_client = Some(client);
            tracing::info!("LSP client initialized successfully");

            // Start file watcher
            self.start_file_watcher()?;
        } else {
            tracing::warn!("No supported language detected, LSP not initialized");
        }

        Ok(())
    }

    /// Start file watcher for the workspace
    fn start_file_watcher(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::unbounded_channel::<FileChangeEvent>();

        match FileWatcher::new(&self.workspace_root, tx) {
            Ok(watcher) => {
                self.file_watcher = Some(watcher);
                self.change_receiver = Some(rx);
                tracing::info!("File watcher started for workspace");
            }
            Err(e) => {
                tracing::warn!("Failed to start file watcher: {}", e);
            }
        }

        Ok(())
    }

    /// Run the MCP server in stdio mode
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting MCP server: {} v{}", self.name, self.version);

        // Initialize LSP first
        self.init_lsp().await?;

        // Spawn file change handler if watcher is active
        let change_handler = if let Some(mut rx) = self.change_receiver.take() {
            let client = self.lsp_client.as_mut().map(|c| c as *mut LspClient);
            Some(tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    tracing::debug!("File change: {:?} - {:?}", event.path, event.kind);
                    // For now just log - full implementation would require Arc<Mutex<LspClient>>
                }
            }))
        } else {
            None
        };

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line?;
            tracing::debug!("Received MCP request: {}", line);

            let request: McpRequest = serde_json::from_str(&line)?;

            let response = self.handle_request(request).await;

            let response_json = serde_json::to_string(&response)?;
            tracing::debug!("Sending MCP response: {}", response_json);

            stdout_lock.write_all(response_json.as_bytes())?;
            stdout_lock.write_all(b"\n")?;
            stdout_lock.flush()?;
        }

        // Cleanup
        if let Some(handler) = change_handler {
            handler.abort();
        }
        if let Some(client) = &mut self.lsp_client {
            client.shutdown().await;
        }
        if let Some(mut watcher) = self.file_watcher.take() {
            watcher.stop();
        }

        Ok(())
    }

    /// Handle an MCP request
    async fn handle_request(&mut self, request: McpRequest) -> McpResponse {
        tracing::info!("Handling MCP request: {}", request.method);

        match request.method.as_str() {
            "initialize" => {
                McpResponse {
                    jsonrpc: "2.0",
                    id: request.id.unwrap_or(0),
                    result: Some(self.handle_initialize()),
                    error: None,
                }
            }
            "initialized" => {
                McpResponse {
                    jsonrpc: "2.0",
                    id: request.id.unwrap_or(0),
                    result: Some(serde_json::json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                McpResponse {
                    jsonrpc: "2.0",
                    id: request.id.unwrap_or(0),
                    result: Some(serde_json::json!({
                        "tools": McpTool::definitions()
                    })),
                    error: None,
                }
            }
            "tools/call" => {
                let params = request.params.clone().unwrap_or(serde_json::json!({}));
                let tool_name = params.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let arguments = params.get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                match self.execute_tool(tool_name, &arguments).await {
                    Ok(response) => {
                        McpResponse {
                            jsonrpc: "2.0",
                            id: request.id.unwrap_or(0),
                            result: Some(response),
                            error: None,
                        }
                    }
                    Err(e) => {
                        McpResponse {
                            jsonrpc: "2.0",
                            id: request.id.unwrap_or(0),
                            result: None,
                            error: Some(McpError {
                                code: -1,
                                message: e.to_string(),
                                data: None,
                            }),
                        }
                    }
                }
            }
            _ => {
                McpResponse {
                    jsonrpc: "2.0",
                    id: request.id.unwrap_or(0),
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }
            }
        }
    }

    /// Execute a tool call with LSP client
    async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let client = self.lsp_client.as_mut()
            .ok_or("LSP client not initialized")?;

        match tool_name {
            "lsp_goto_definition" => {
                let file_path = arguments["file_path"].as_str().unwrap_or("");
                let line = arguments["line"].as_u64().unwrap_or(0) as u32;
                let character = arguments["character"].as_u64().unwrap_or(0) as u32;

                // Open document first - rust-analyzer needs this for references/definitions
                client.open_document(file_path).await?;

                let result = client.text_document_request(
                    "textDocument/definition",
                    file_path,
                    line,
                    character
                ).await?;

                // Parse result and extract code snippet
                self.parse_definition_result(result, arguments["all_implementations"].as_bool().unwrap_or(false))
            }
            "lsp_find_references" => {
                let file_path = arguments["file_path"].as_str().unwrap_or("");
                let line = arguments["line"].as_u64().unwrap_or(0) as u32;
                let character = arguments["character"].as_u64().unwrap_or(0) as u32;

                // Open document first - rust-analyzer needs this for references/definitions
                client.open_document(file_path).await?;

                let result = client.find_references(file_path, line, character).await?;

                self.parse_references_result(result)
            }
            "lsp_hover" => {
                let file_path = arguments["file_path"].as_str().unwrap_or("");
                let line = arguments["line"].as_u64().unwrap_or(0) as u32;
                let character = arguments["character"].as_u64().unwrap_or(0) as u32;

                // Open document first - rust-analyzer needs this
                client.open_document(file_path).await?;

                let result = client.text_document_request(
                    "textDocument/hover",
                    file_path,
                    line,
                    character
                ).await?;

                self.parse_hover_result(result)
            }
            "lsp_workspace_symbols" => {
                let query = arguments["query"].as_str().unwrap_or("");

                let result = client.workspace_symbols(query).await?;

                self.parse_symbols_result(result)
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
            "lsp_hybrid_search" => {
                let query = arguments["query"].as_str().unwrap_or("");
                let include_symbols = arguments["include_symbols"].as_bool().unwrap_or(true);
                let include_text = arguments["include_text"].as_bool().unwrap_or(true);
                let max_results = arguments["max_results"].as_u64().unwrap_or(10) as usize;

                // Parse file_types if provided
                let file_types = arguments["file_types"].as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    });

                let options = crate::search::HybridSearchOptions {
                    query: query.to_string(),
                    include_symbols,
                    include_text,
                    file_types,
                    max_results,
                };

                let workspace_root = self.workspace_root.to_string_lossy().to_string();
                let result = crate::search::hybrid_search(
                    self.lsp_client.as_mut(),
                    &workspace_root,
                    &options
                ).await?;

                self.parse_hybrid_result(result)
            }
            _ => {
                Err(format!("Unknown tool: {}", tool_name).into())
            }
        }
    }

    /// Parse definition result and extract snippets
    fn parse_definition_result(
        &self,
        result: serde_json::Value,
        all_implementations: bool,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        use crate::bridge::snippet::extract_snippet;

        tracing::debug!("Parsing definition result: {:?}", result);

        let mut locations: Vec<(String, u32)> = Vec::new();

        // Handle single location (GotoDefinitionResponse can be Location or LocationLink)
        if let Some(uri) = result.get("uri").and_then(|u| u.as_str()) {
            let path = Self::uri_to_path(uri).unwrap_or(uri.to_string());
            let line = result.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(|l| l.as_u64())
                .unwrap_or(0) as u32;
            locations.push((path, line));
        }

        // Handle LocationLink format (targetUri, targetRange)
        if let Some(uri) = result.get("targetUri").and_then(|u| u.as_str()) {
            let path = Self::uri_to_path(uri).unwrap_or(uri.to_string());
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
                // Location format
                if let Some(uri) = loc.get("uri").and_then(|u| u.as_str()) {
                    let path = Self::uri_to_path(uri).unwrap_or(uri.to_string());
                    let line = loc.get("range")
                        .and_then(|r| r.get("start"))
                        .and_then(|s| s.get("line"))
                        .and_then(|l| l.as_u64())
                        .unwrap_or(0) as u32;
                    locations.push((path, line));
                }
                // LocationLink format
                if let Some(uri) = loc.get("targetUri").and_then(|u| u.as_str()) {
                    let path = Self::uri_to_path(uri).unwrap_or(uri.to_string());
                    let line = loc.get("targetRange")
                        .and_then(|r| r.get("start"))
                        .and_then(|s| s.get("line"))
                        .and_then(|l| l.as_u64())
                        .unwrap_or(0) as u32;
                    locations.push((path, line));
                }
            }
        }

        tracing::debug!("Found {} locations", locations.len());

        // Limit to top 3 unless all_implementations is true
        let max = if all_implementations { locations.len() } else { 3 };
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
    fn parse_references_result(&self, result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut references: Vec<String> = Vec::new();

        if let Some(arr) = result.as_array() {
            for loc in arr {
                if let Some(uri) = loc.get("uri").and_then(|u| u.as_str()) {
                    let path = Self::uri_to_path(uri).unwrap_or(uri.to_string());
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
    fn parse_hover_result(&self, result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        tracing::debug!("Parsing hover result: {:?}", result);

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

    /// Parse workspace symbols result
    fn parse_symbols_result(&self, result: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut symbols: Vec<String> = Vec::new();

        if let Some(arr) = result.as_array() {
            for sym in arr {
                let name = sym.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                let kind = sym.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
                let kind_name = match kind {
                    5 => "Class",
                    6 => "Method",
                    9 => "Constructor",
                    10 => "Enum",
                    11 => "Interface",
                    12 => "Function",
                    13 => "Variable",
                    14 => "Constant",
                    22 => "EnumMember",
                    23 => "Struct",
                    24 => "Event",
                    25 => "Operator",
                    26 => "TypeParameter",
                    _ => "Symbol"
                };

                let location = sym.get("location");
                let path = location
                    .and_then(|l| l.get("uri"))
                    .and_then(|u| u.as_str())
                    .and_then(|u| Self::uri_to_path(u))
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

    /// Parse hybrid search result
    fn parse_hybrid_result(&self, result: crate::search::HybridResult) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut lines: Vec<String> = Vec::new();

        // Add symbol results first
        if !result.symbol_results.is_empty() {
            lines.push(format!("=== Symbols ({} found) ===", result.symbol_results.len()));
            for sym in &result.symbol_results {
                let desc = sym.description.as_deref().unwrap_or("");
                if desc.is_empty() {
                    lines.push(format!("{} ({}) @ {}:{}", sym.name, sym.kind, sym.file, sym.line));
                } else {
                    lines.push(format!("{} ({}) @ {}:{} - {}", sym.name, sym.kind, sym.file, sym.line, desc));
                }
            }
        }

        // Add text results
        if !result.text_results.is_empty() {
            if !lines.is_empty() {
                lines.push("".to_string());
            }
            lines.push(format!("=== Text Matches ({} found) ===", result.text_results.len()));
            for txt in &result.text_results {
                lines.push(format!("{}:{} | {}", txt.file, txt.line, txt.content.trim()));
            }
        }

        if lines.is_empty() {
            return Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": "No results found"
                }]
            }));
        }

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": lines.join("\n")
            }]
        }))
    }

    /// Convert file URI to path, handling Windows file:///d: format
    pub fn uri_to_path(uri: &str) -> Option<String> {
        let without_prefix = uri.strip_prefix("file://")?;

        // Handle Windows paths like /d:/path or /D:/path
        if without_prefix.len() >= 3 && without_prefix.as_bytes()[0] == b'/' {
            let drive = without_prefix.as_bytes()[1];
            if drive.is_ascii_alphabetic() && without_prefix.as_bytes()[2] == b':' {
                // Windows path: /d:/path -> d:/path
                return Some(without_prefix[1..].to_string());
            }
        }

        Some(without_prefix.to_string())
    }

    /// Handle MCP initialize request
    pub fn handle_initialize(&self) -> serde_json::Value {
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.name,
                "version": self.version
            }
        })
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}