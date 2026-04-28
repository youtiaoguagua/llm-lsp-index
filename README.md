# LSP-to-MCP Bridge

A headless LSP (Language Server Protocol) client that exposes IDE capabilities to AI models via MCP (Model Context Protocol).

## Quick Start (npm)

**Install globally:**
```bash
npm install -g lsp-index-mcp
```

**Or use with npx (no installation):**
```bash
npx -y lsp-index-mcp /path/to/your/project
```

The install script automatically downloads:
- `lsp-index` binary for your platform
- JDT Language Server (for Java)
- gopls (for Go, if Go is installed)
- typescript-language-server (for TypeScript, if npm is installed)

## MCP Configuration

**Claude Desktop (`claude_desktop_config.json`):**

```json
{
  "mcpServers": {
    "lsp-index": {
      "command": "npx",
      "args": ["-y", "lsp-index-mcp", "/path/to/your/project"]
    }
  }
}
```

**With environment variable:**
```json
{
  "mcpServers": {
    "lsp-index": {
      "command": "npx",
      "args": ["-y", "lsp-index-mcp"],
      "env": {
        "LSP_INDEX_WORKSPACE": "/path/to/your/project"
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
      "command": ["npx", "-y", "lsp-index-mcp", "."],
      "transport": "stdio"
    }
  ]
}
```

## Features

- **Multi-Language Support**: Rust (rust-analyzer), Java (JDT LS), Go (gopls), TypeScript (typescript-language-server)
- **Hybrid Search**: Combines LSP symbol search with ripgrep-like text search
- **LSP Tools Exposed via MCP**:
  - `lsp_hybrid_search` - Search symbols and text across workspace
  - `lsp_goto_definition` - Jump to symbol definitions
  - `lsp_find_references` - Find all references to a symbol
  - `lsp_hover` - Get type information and documentation
  - `lsp_workspace_symbols` - Search symbols across workspace
  - `lsp_get_diagnostics` - Get lint/type errors
  - `text_search` - Text search (works without LSP)
  - `file_list` - List files (works without LSP)

## Prerequisites

- **Rust projects**: `rustup component add rust-analyzer`
- **Java projects**: Automatic (JDT LS installed by npm package)
- **Go projects**: Go must be installed
- **TypeScript projects**: npm must be installed

## Architecture

```
┌─────────────┐     MCP Protocol      ┌─────────────────┐     LSP Protocol     ┌─────────────┐
│   Claude    │ ←───────────────────→ │   MCP Server    │ ←──────────────────→ │  LSP Server │
│   (AI)      │   (stdio/jsonrpc)     │  (lsp-index)    │   (stdio/jsonrpc)  │(rust-analyzer│
└─────────────┘                       └─────────────────┘                      │  gopls, etc)│
                                                      │                        └─────────────┘
                                                      ↓
                                               ┌──────────────┐
                                               │ File Watcher │
                                               └──────────────┘
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `LSP_INDEX_WORKSPACE` | Project directory path (alternative to CLI arg) |
| `RUST_LOG` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `JDT_LS_PATH` | Path to JDT LS jar (optional, auto-detected) |

## Development

To build from source:

```bash
# Prerequisites
- Rust toolchain (latest stable)

# Build CLI
cargo build --release

# Build HTTP server (optional)
cargo build --release --bin lsp-index-http

# Build GUI (optional)
cargo tauri build
```

## License

MIT
