# LSP-to-MCP Bridge

A headless LSP (Language Server Protocol) client that exposes IDE capabilities to AI models via MCP (Model Context Protocol).

## Overview

This project bridges the gap between traditional IDE features (go-to-definition, find-references, hover info) and AI assistants. It allows AI models like Claude to navigate codebases with the same precision as a human developer using an IDE.

## Features

- **Multi-Language Support**: Rust (rust-analyzer), Go (gopls), TypeScript (typescript-language-server)
- **Hybrid Search**: Combines LSP symbol search with ripgrep-like text search to find TODOs, comments, and code
- **LSP Tools Exposed via MCP**:
  - `lsp_hybrid_search` - Search symbols and text (TODOs, comments) across workspace
  - `lsp_goto_definition` - Jump to symbol definitions with code snippets
  - `lsp_find_references` - Find all references to a symbol
  - `lsp_hover` - Get type information and documentation
  - `lsp_workspace_symbols` - Search symbols across the entire workspace
  - `lsp_get_diagnostics` - Get lint/type errors
- **File Watching**: Automatic synchronization of file changes to LSP
- **Desktop GUI**: Tauri-based management interface
- **Dual Transport**: stdio (MCP standard) and HTTP (StreamableHTTP)

## Architecture

```
┌─────────────┐     MCP Protocol      ┌─────────────────┐     LSP Protocol     ┌─────────────┐
│   Claude    │ ←───────────────────→ │   MCP Server    │ ←──────────────────→ │  LSP Server │
│   (AI)      │   (stdio/jsonrpc)     │  (lsp-index)    │   (stdio/jsonrpc)  │(rust-analyzer│
└─────────────┘        or             └─────────────────┘                      │  gopls, etc)│
                       HTTP                    │                                 └─────────────┘
                       (3000)                 ↓
                                        ┌──────────────┐
                                        │ File Watcher │     ┌──────────────┐
                                        │  (notify)    │     │ Text Search  │
                                        └──────────────┘     │ (ignore+grep)│
                                                             └──────────────┘
```

## Quick Start

### Prerequisites

- Rust toolchain (latest stable)
- LSP servers for your languages:
  - Rust: `rustup component add rust-analyzer`
  - Go: `go install golang.org/x/tools/gopls@latest`
  - TypeScript: `npm install -g typescript-language-server`

### Build

```bash
# Build CLI
cargo build --release

# Build HTTP server (optional)
cargo build --release --bin lsp-index-http

# Build GUI (optional)
cargo tauri build
```

### Usage

#### MCP Configuration

Configure your MCP client (Claude Desktop, Claude Code, etc.) to use lsp-index:

**Claude Desktop (`claude_desktop_config.json`):**

```json
{
  "mcpServers": {
    "lsp-index": {
      "command": "/path/to/lsp-index",
      "args": ["/path/to/your/project"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Claude Code (`.mcp.json` in project root):**

```json
{
  "servers": [
    {
      "name": "lsp-index",
      "command": ["/path/to/lsp-index", "."],
      "transport": "stdio"
    }
  ]
}
```

**Environment Variables:**

| Variable | Description |
|----------|-------------|
| `LSP_INDEX_WORKSPACE` | Project directory path (alternative to CLI arg) |
| `RUST_LOG` | Log level: `error`, `warn`, `info`, `debug`, `trace` |

#### CLI Mode (stdio MCP Server)

```bash
# Run in a project directory
cd my-rust-project
./target/release/lsp-index

# Or specify workspace via argument
./target/release/lsp-index /path/to/project

# Or use environment variable
export LSP_INDEX_WORKSPACE=/path/to/project
./target/release/lsp-index
```

The server reads MCP requests from stdin and writes responses to stdout.

#### HTTP Mode (StreamableHTTP MCP Server)

For MCP clients that support HTTP transport:

```bash
# Start HTTP server
./target/release/lsp-index-http

# Or with custom host/port
./target/release/lsp-index-http --host 127.0.0.1 --port 3000
```

Endpoint: `POST http://127.0.0.1:3000/mcp` - MCP protocol over HTTP

#### Test with manual MCP request

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lsp_workspace_symbols","arguments":{"query":"MyStruct"}}}' | ./target/release/lsp-index
```

#### GUI Mode

```bash
./src-tauri/target/release/lsp-index-gui.exe
```

## MCP Tools

### lsp_hybrid_search

Search for symbols and text matches across the workspace. Combines LSP symbol search with ripgrep-like text search to find TODOs, comments, strings, and code symbols.

**Parameters:**
- `query` (required): Search pattern (symbol name or text)
- `include_symbols`: Include LSP symbol results (default: `true`)
- `include_text`: Include text search results (default: `true`)
- `file_types`: Filter by file extensions, e.g., `["rs", "toml"]` (optional)
- `max_results`: Maximum total results (default: `10`)

**Example:**
```json
{
  "name": "lsp_hybrid_search",
  "arguments": {
    "query": "TODO",
    "include_symbols": false,
    "include_text": true,
    "file_types": ["rs"]
  }
}
```

**Output:**
```
=== Symbols (2 found) ===
parse_definition_result (Function) @ src/mcp/server.rs:360
hybrid_search (Function) @ src/search/hybrid.rs:66

=== Text Matches (3 found) ===
src/main.rs:15 | // TODO: Add better error handling
src/lib.rs:42 | // TODO: Support more languages
```

### lsp_goto_definition

Find the definition of a symbol at a specific position.

**Parameters:**
- `file_path`: Absolute path to the source file
- `line`: Line number (0-indexed)
- `character`: Character position (0-indexed)
- `all_implementations`: (optional) Return all implementations, not just top 3

**Example:**
```json
{
  "name": "lsp_goto_definition",
  "arguments": {
    "file_path": "/home/user/project/src/main.rs",
    "line": 10,
    "character": 5
  }
}
```

### lsp_find_references

Find all references to a symbol.

**Parameters:**
- `file_path`: Absolute path to the source file
- `line`: Line number (0-indexed)
- `character`: Character position (0-indexed)

### lsp_hover

Get hover information (type signature, documentation) for a symbol.

**Parameters:**
- `file_path`: Absolute path to the source file
- `line`: Line number (0-indexed)
- `character`: Character position (0-indexed)

### lsp_workspace_symbols

Search for symbols across the entire workspace.

**Parameters:**
- `query`: Symbol name to search for

### lsp_get_diagnostics

Get diagnostics (errors, warnings) for a file.

**Parameters:**
- `file_path`: Absolute path to the source file

## Development

### Test Fixtures

Test fixtures are in `tests/fixtures/`:
- `rust-sample/`: Example Rust project
- `go-sample/`: Example Go project

### Adding a New Language

1. Add LSP configuration in `src/lsp/registry.rs`:

```rust
pub fn mylang() -> Self {
    Self {
        language: "mylang".to_string(),
        binary_name: "mylang-lsp".to_string(),
        binary_path: None,
        root_file: "mylang.toml".to_string(),
    }
}
```

2. Add to `LspRegistry::new()`:

```rust
Self {
    configs: vec![
        LspConfig::rust(),
        LspConfig::go(),
        LspConfig::typescript(),
        LspConfig::mylang(),  // Add here
    ],
}
```

## Configuration

The project auto-detects language based on root files:

| Language | Root File | LSP Server |
|----------|-----------|------------|
| Rust | `Cargo.toml` | rust-analyzer |
| Go | `go.mod` | gopls |
| TypeScript | `package.json` | typescript-language-server |

## License

MIT

## Contributing

See [PLAN.md](PLAN.md) for the implementation roadmap.
