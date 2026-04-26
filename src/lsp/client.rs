//! Headless LSP Client - manages communication with LSP server
//!
//! Handles initialize handshake, file open notifications, and requests.

use crate::lsp::process::LspProcess;
use crate::lsp::registry::LspConfig;
use lsp_types::{ClientCapabilities, InitializeParams, InitializedParams};
use std::collections::HashMap;
use url::Url;

/// Diagnostics cache entry
#[derive(Debug, Clone)]
pub struct DiagnosticsEntry {
    pub uri: String,
    pub diagnostics: Vec<serde_json::Value>,
    pub timestamp: std::time::Instant,
}

/// LSP Client state
pub struct LspClient {
    /// LSP process
    process: LspProcess,
    /// Language name
    language: String,
    /// Whether handshake is complete
    initialized: bool,
    /// Workspace root URI
    workspace_root: String,
    /// Diagnostics cache (URI -> diagnostics)
    diagnostics_cache: HashMap<String, DiagnosticsEntry>,
}

impl LspClient {
    /// Create a new LSP client
    pub async fn new(config: &LspConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let process = LspProcess::spawn(config).await?;

        tracing::info!("Created LSP client for language: {}", config.language);

        Ok(Self {
            process,
            language: config.language.clone(),
            initialized: false,
            workspace_root: "".to_string(),
            diagnostics_cache: HashMap::new(),
        })
    }

    /// Perform LSP handshake (initialize + initialized)
    pub async fn initialize(&mut self, workspace_root: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initializing LSP client for workspace: {}", workspace_root);

        self.workspace_root = workspace_root.to_string();

        // Prepare initialize params - use absolute path for Windows
        let abs_path = std::fs::canonicalize(workspace_root)
            .map_err(|e| format!("Failed to resolve workspace path: {}", e))?;

        tracing::info!("Resolved workspace path: {}", abs_path.display());

        // Convert to URL format
        // On Windows, canonicalize returns \\?\ prefix which needs to be removed for URL
        let path_str = abs_path.to_string_lossy();
        let clean_path = if path_str.starts_with("\\\\?\\") {
            // Remove \\?\ prefix (UNC path format)
            path_str[4..].replace('\\', "/")
        } else {
            path_str.replace('\\', "/")
        };

        // Use url crate to properly format file URL
        let root_uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid workspace URI: {}", e))?;

        tracing::info!("Workspace URI: {}", root_uri);

        tracing::debug!("Initialize params: root_uri={}", root_uri);

        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri),
            capabilities: ClientCapabilities::default(),
            ..InitializeParams::default()
        };

        // Send initialize request
        let params_json = serde_json::to_value(params)?;
        self.process.send_request("initialize", params_json).await?;

        // Read response
        let response = self.process.read_response().await?;

        tracing::debug!("LSP initialize response: {:?}", response);

        // Check for error in response
        if let Some(error) = response.get("error") {
            return Err(format!("LSP initialize error: {:?}", error).into());
        }

        // Send initialized notification
        let initialized_params = InitializedParams {};
        self.process.send_notification("initialized", serde_json::to_value(initialized_params)?).await?;

        self.initialized = true;
        tracing::info!("LSP handshake complete for {}", self.language);

        // Wait for rust-analyzer to finish initial indexing
        // This is a simple heuristic - wait a bit for the analysis to complete
        // For larger projects, this may need to be longer or use a more sophisticated approach
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    /// Open a text document in LSP
    pub async fn open_document(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!("Opening document: {}", file_path);

        // Convert path to proper URL format
        let clean_path = file_path.replace('\\', "/");
        let uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid file URI: {}", e))?;

        tracing::debug!("Document URI: {}", uri);

        // Read file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.to_string(),
                "languageId": self.language,
                "version": 0,
                "text": content
            }
        });

        self.process.send_notification("textDocument/didOpen", params).await?;

        // Wait for LSP to process the document - rust-analyzer needs time
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    /// Send textDocument/didChange notification
    pub async fn send_did_change(
        &mut self,
        file_path: &str,
        _content_changes: Option<Vec<serde_json::Value>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!("Sending didChange for: {}", file_path);

        // Convert path to proper URL format
        let clean_path = file_path.replace('\\', "/");
        let uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid file URI: {}", e))?;

        // For now, send full document content as change (simpler approach)
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.to_string(),
                "version": 1
            },
            "contentChanges": [{
                "text": content
            }]
        });

        self.process.send_notification("textDocument/didChange", params).await?;
        tracing::debug!("didChange sent for: {}", file_path);

        Ok(())
    }

    /// Send textDocument/didClose notification
    pub async fn send_did_close(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!("Sending didClose for: {}", file_path);

        // Convert path to proper URL format
        let clean_path = file_path.replace('\\', "/");
        let uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid file URI: {}", e))?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.to_string()
            }
        });

        self.process.send_notification("textDocument/didClose", params).await?;
        tracing::debug!("didClose sent for: {}", file_path);

        Ok(())
    }

    /// Send a textDocument request (definition, references, hover, etc.)
    pub async fn text_document_request(
        &mut self,
        method: &str,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Convert path to proper URL format
        let clean_path = file_path.replace('\\', "/");
        let uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid file URI: {}", e))?;

        tracing::debug!("Request URI: {}", uri);

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.to_string()
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        self.process.send_request(method, params).await?;
        let response = self.process.read_response().await?;

        // Extract result from response
        if let Some(error) = response.get("error") {
            return Err(format!("LSP {} error: {:?}", method, error).into());
        }

        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Send textDocument/references request (needs context field)
    pub async fn find_references(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Convert path to proper URL format
        let clean_path = file_path.replace('\\', "/");
        let uri = Url::parse(&format!("file:///{}", clean_path))
            .map_err(|e| format!("Invalid file URI: {}", e))?;

        tracing::debug!("References request URI: {}", uri);

        // references request needs context field
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.to_string()
            },
            "position": {
                "line": line,
                "character": character
            },
            "context": {
                "includeDeclaration": true
            }
        });

        self.process.send_request("textDocument/references", params).await?;
        let response = self.process.read_response().await?;

        // Extract result from response
        if let Some(error) = response.get("error") {
            return Err(format!("LSP references error: {:?}", error).into());
        }

        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Send workspace/symbol request
    pub async fn workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let params = serde_json::json!({
            "query": query
        });

        self.process.send_request("workspace/symbol", params).await?;
        let response = self.process.read_response().await?;

        if let Some(error) = response.get("error") {
            return Err(format!("LSP workspace/symbol error: {:?}", error).into());
        }

        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Check if client is ready for requests
    pub fn is_initialized(&mut self) -> bool {
        self.initialized && self.process.is_running()
    }

    /// Get cached diagnostics for a file
    pub fn get_diagnostics(&self, file_path: &str) -> Vec<serde_json::Value> {
        let clean_path = file_path.replace('\\', "/");
        let uri = format!("file:/// {}", clean_path);

        self.diagnostics_cache
            .get(&uri)
            .map(|entry| entry.diagnostics.clone())
            .unwrap_or_default()
    }

    /// Update diagnostics cache from LSP notification
    pub fn update_diagnostics(&mut self, uri: String, diagnostics: Vec<serde_json::Value>) {
        self.diagnostics_cache.insert(uri.clone(), DiagnosticsEntry {
            uri,
            diagnostics,
            timestamp: std::time::Instant::now(),
        });
    }

    /// Send a custom LSP request (for language-specific extensions like JDT LS)
    pub async fn send_custom_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.process.send_request(method, params).await?;
        let response = self.process.read_response().await?;

        if let Some(error) = response.get("error") {
            return Err(format!("LSP {} error: {:?}", method, error).into());
        }

        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Shutdown the LSP client
    pub async fn shutdown(&mut self) {
        self.process.kill().await;
        self.initialized = false;
    }
}