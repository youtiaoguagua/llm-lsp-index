# LSP-to-MCP Bridge

A headless LSP (Language Server Protocol) client that exposes IDE capabilities to AI models via MCP (Model Context Protocol).

## Overview

This project bridges the gap between traditional IDE features (go-to-definition, find-references, hover info) and AI assistants. It allows AI models like Claude to navigate codebases with the same precision as a human developer using an IDE.

## Features

- **Multi-Language Support**: Rust (rust-analyzer), Go (gopls), TypeScript (typescript-language-server)
- **LSP Tools Exposed via MCP**:
  - `lsp_goto_definition` - Jump to symbol definitions with code snippets
  - `lsp_find_references` - Find all references to a symbol
  - `lsp_hover` - Get type information and documentation
  - `lsp_workspace_symbols` - Search symbols across the entire workspace
  - `lsp_get_diagnostics` - Get lint/type errors (framework ready)
- **File Watching**: Automatic synchronization of file changes to LSP
- **Desktop GUI**: Tauri-based management interface

## Architecture

```
┌─────────────┐     MCP Protocol      ┌─────────────────┐     LSP Protocol     ┌─────────────┐
│   Claude    │ ←───────────────────→ │   MCP Server    │ ←──────────────────→ │  LSP Server │
│   (AI)      │   (stdio/jsonrpc)     │  (lsp-index)    │   (stdio/jsonrpc)  │(rust-analyzer|
└─────────────┘                       └─────────────────┘                      │  gopls, etc)│
                                               │                               └─────────────┘
                                               ↓
                                        ┌──────────────┐
                                        │ File Watcher │
                                        │  (notify)    │
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

# Build GUI (optional)
cargo tauri build
```

### Usage

#### CLI Mode (stdio MCP Server)

```bash
# Run in a project directory
cd my-rust-project
./target/release/lsp-index

# Or specify workspace
./target/release/lsp-index --workspace /path/to/project
```

The server reads MCP requests from stdin and writes responses to stdout.

#### Test with manual MCP request

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lsp_workspace_symbols","arguments":{"query":"MyStruct"}}}' | ./target/release/lsp-index
```

#### GUI Mode

```bash
./src-tauri/target/release/lsp-index-gui.exe
```

## MCP Tools

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

## Project Structure

```
.
├── Cargo.toml              # Main crate configuration
├── src/
│   ├── main.rs            # CLI entry point
│   ├── lib.rs             # Library exports
│   ├── config.rs          # Configuration management
│   ├── lsp/               # LSP client implementation
│   │   ├── client.rs      # LSP client (handshake, requests)
│   │   ├── process.rs     # LSP process management
│   │   ├── registry.rs    # Language detection & LSP config
│   │   ├── watcher.rs     # File system watcher
│   │   └── types.rs       # LSP types
│   ├── mcp/               # MCP server implementation
│   │   ├── server.rs      # MCP protocol handler
│   │   ├── protocol.rs    # MCP types
│   │   └── tools.rs       # Tool definitions
│   ├── bridge/            # LSP-to-MCP mapping
│   │   ├── handlers.rs    # Tool call handlers
│   │   └── snippet.rs     # Code snippet extraction
│   └── utils/             # Utilities
│       ├── file.rs
│       └── uri.rs
├── src-tauri/             # Tauri GUI
│   ├── src/
│   │   ├── main.rs
│   │   └── lib.rs         # Tauri commands
│   ├── static/
│   │   └── index.html     # Frontend
│   └── tauri.conf.json    # Tauri configuration
├── tests/
│   └── fixtures/          # Test projects
│       ├── rust-sample/
│       └── go-sample/
├── PLAN.md                # Implementation plan
└── SPEC.md                # Architecture specification
```

## Development

### Running Tests

```bash
# Build and test
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

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
