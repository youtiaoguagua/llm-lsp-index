//! LSP process management - spawn and communicate with LSP servers

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::lsp::registry::LspConfig;

/// LSP process wrapper with stdin/stdout communication
pub struct LspProcess {
    /// The child process
    process: Option<Child>,
    /// stdin writer for sending requests
    stdin: Option<BufWriter<ChildStdin>>,
    /// stdout reader for receiving responses
    stdout: Option<BufReader<ChildStdout>>,
    /// Binary name for logging
    binary_name: String,
    /// Request ID counter
    request_id: u64,
}

impl LspProcess {
    /// Spawn a new LSP process
    pub async fn spawn(config: &LspConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let binary_name = config.binary_name.clone();
        let spawn_args = config.get_spawn_command();

        tracing::info!("Starting LSP process: {}", spawn_args.join(" "));

        let mut process = if spawn_args.len() > 1 {
            Command::new(&spawn_args[0])
                .args(&spawn_args[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            Command::new(&binary_name)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let stdin = process.stdin.take()
            .map(|s| BufWriter::new(s));
        let stdout = process.stdout.take()
            .map(|s| BufReader::new(s));

        tracing::info!("LSP process started: {}", binary_name);

        Ok(Self {
            process: Some(process),
            stdin,
            stdout,
            binary_name,
            request_id: 0,
        })
    }

    /// Send a JSON-RPC request to LSP
    pub async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let id = self.request_id;
        self.request_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        if let Some(stdin) = &mut self.stdin {
            stdin.write_all(message.as_bytes()).await?;
            stdin.flush().await?;
            tracing::debug!("Sent LSP request: {} (id={})", method, id);
        } else {
            return Err("stdin not available".into());
        }

        Ok(id)
    }

    /// Send a JSON-RPC notification (no id, no response expected)
    pub async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let content = serde_json::to_string(&notification)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        if let Some(stdin) = &mut self.stdin {
            stdin.write_all(message.as_bytes()).await?;
            stdin.flush().await?;
            tracing::debug!("Sent LSP notification: {}", method);
        }

        Ok(())
    }

    /// Read a response from LSP stdout (skipping notifications)
    pub async fn read_response(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        if let Some(stdout) = &mut self.stdout {
            loop {
                // LSP uses base header format: headers separated by \r\n, followed by \r\n\r\n, then content
                // Read headers until we find Content-Length
                let mut content_length: Option<usize> = None;

                loop {
                    let mut header_line = String::new();
                    stdout.read_line(&mut header_line).await?;

                    // Empty line signals end of headers
                    if header_line == "\r\n" || header_line == "\n" || header_line.is_empty() {
                        break;
                    }

                    // Parse Content-Length header
                    if header_line.to_lowercase().starts_with("content-length:") {
                        let value = header_line
                            .split(':')
                            .nth(1)
                            .and_then(|s| s.trim().parse().ok());
                        content_length = value;
                        tracing::debug!("Found Content-Length: {:?}", content_length);
                    }
                }

                let content_length = content_length.ok_or("No Content-Length header found")?;

                // Read the JSON content
                let mut content_buf = vec![0u8; content_length];
                stdout.read_exact(&mut content_buf).await?;

                let content = String::from_utf8(content_buf)?;
                tracing::trace!("LSP raw content: {}", content);
                let message: serde_json::Value = serde_json::from_str(&content)?;

                // Check if this is a notification (has method field) - skip it
                if message.get("method").is_some() {
                    tracing::debug!("Skipping LSP notification: {}", message.get("method").unwrap());
                    continue; // Read next message
                }

                // This is a response - return it
                tracing::debug!("Received LSP response: {:?}", message);
                return Ok(message);
            }
        } else {
            Err("stdout not available".into())
        }
    }

    /// Check if process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(process) = &mut self.process {
            process.try_wait().map(|r| r.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    /// Kill the LSP process
    pub async fn kill(&mut self) {
        if let Some(process) = &mut self.process {
            process.kill().await.ok();
            tracing::info!("Killed LSP process: {}", self.binary_name);
        }
        self.process = None;
        self.stdin = None;
        self.stdout = None;
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Use start_kill() for non-blocking kill in Drop
            process.start_kill().ok();
        }
    }
}