//! MCP (Model Context Protocol) integration tests

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use serde_json::json;

/// Test MCP protocol request/response parsing
#[test]
fn test_mcp_protocol_parsing() {
    use lsp_index::mcp::protocol::{McpRequest, McpResponse, McpError};

    // Test parsing an initialize request
    let request_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let request: McpRequest = serde_json::from_value(request_json.clone()).expect("Failed to parse MCP request");
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.id, Some(1));
    assert_eq!(request.method, "initialize");

    // Test constructing a response
    let response = McpResponse {
        jsonrpc: "2.0",
        id: 1,
        result: Some(json!({"protocolVersion": "2024-11-05"})),
        error: None,
    };

    let response_json = serde_json::to_value(&response).expect("Failed to serialize response");
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 1);
    assert!(response_json["result"].is_object());
}

/// Test MCP tool definitions are valid
#[test]
fn test_mcp_tool_definitions() {
    use lsp_index::mcp::tools::McpTool;

    let definitions = McpTool::definitions();

    // Check all expected tools are defined
    let tool_names: Vec<String> = definitions
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_string())
        .collect();

    assert!(tool_names.contains(&"lsp_goto_definition".to_string()),
        "Missing lsp_goto_definition tool");
    assert!(tool_names.contains(&"lsp_find_references".to_string()),
        "Missing lsp_find_references tool");
    assert!(tool_names.contains(&"lsp_hover".to_string()),
        "Missing lsp_hover tool");
    assert!(tool_names.contains(&"lsp_workspace_symbols".to_string()),
        "Missing lsp_workspace_symbols tool");
    assert!(tool_names.contains(&"lsp_get_diagnostics".to_string()),
        "Missing lsp_get_diagnostics tool");
    assert!(tool_names.contains(&"lsp_hybrid_search".to_string()),
        "Missing lsp_hybrid_search tool");

    // Verify each tool has required schema fields
    for def in &definitions {
        assert!(def["name"].is_string(), "Tool missing name");
        assert!(def["description"].is_string(), "Tool missing description");
        assert!(def["inputSchema"].is_object(), "Tool missing inputSchema");
    }
}

/// Test MCP tool input structure
#[test]
fn test_mcp_tool_structure() {
    use lsp_index::mcp::tools::McpTool;

    // Verify tool names match expected
    let definitions = McpTool::definitions();
    let tool_names: Vec<&str> = definitions
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();

    // Test that tool names are consistent with what execute_tool expects
    assert!(tool_names.contains(&"lsp_goto_definition"));
    assert!(tool_names.contains(&"lsp_find_references"));
    assert!(tool_names.contains(&"lsp_hover"));
    assert!(tool_names.contains(&"lsp_workspace_symbols"));
    assert!(tool_names.contains(&"lsp_get_diagnostics"));
    assert!(tool_names.contains(&"lsp_hybrid_search"));

    // Verify tool schemas have correct field names
    let goto_def = definitions.iter().find(|d| d["name"] == "lsp_goto_definition").unwrap();
    let schema = &goto_def["inputSchema"];
    let required = schema["required"].as_array().unwrap();
    assert!(required.iter().any(|r| r == "file_path"));
    assert!(required.iter().any(|r| r == "line"));
    assert!(required.iter().any(|r| r == "character"));
}

/// Test building and running MCP server binary
#[test]
#[ignore = "Requires building binary first - run manually"]
fn test_mcp_server_stdio() {
    // Build the binary first
    let build_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "lsp-index"])
        .status()
        .expect("Failed to build binary");

    assert!(build_status.success(), "Build failed");

    // Start the MCP server
    let mut child = Command::new("./target/release/lsp-index")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    {
        let mut stdin = stdin;
        writeln!(stdin, "{}", init_request).expect("Failed to write request");
    }

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).expect("Failed to read response");

    let response: serde_json::Value = serde_json::from_str(&response_line)
        .expect("Failed to parse response");

    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    // Send tools/list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    {
        let mut stdin = child.stdin.take().expect("Failed to get stdin");
        writeln!(stdin, "{}", tools_request).expect("Failed to write request");
    }

    let mut tools_response_line = String::new();
    reader.read_line(&mut tools_response_line).expect("Failed to read tools response");

    let tools_response: serde_json::Value = serde_json::from_str(&tools_response_line)
        .expect("Failed to parse tools response");

    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["result"]["tools"].is_array());

    // Cleanup
    child.kill().expect("Failed to kill child process");
}

/// Test MCP server with Rust sample project
#[tokio::test]
#[ignore = "Requires rust-analyzer installed"]
async fn test_mcp_with_rust_project() {
    use lsp_index::mcp::server::McpServer;
    use lsp_index::lsp::LspRegistry;

    let rust_sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rust-sample");

    // Create MCP server
    let mut server = McpServer::new();
    server.set_workspace(rust_sample.clone());

    // Initialize LSP
    let result = server.init_lsp().await;

    // This will fail without rust-analyzer, but we can verify the setup logic
    if result.is_err() {
        println!("LSP init failed (expected if rust-analyzer not installed): {:?}", result);
    }

    // Verify registry detects Rust
    let registry = LspRegistry::new();
    let config = registry.detect_language(&rust_sample);
    assert!(config.is_some(), "Should detect Rust project");
    assert_eq!(config.unwrap().language, "rust");
}
