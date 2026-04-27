# lsp-index-mcp

LSP-to-MCP Bridge - Headless LSP Client exposing IDE capabilities to AI via the Model Context Protocol (MCP).

## Installation

```bash
npm install -g lsp-index-mcp
```

Or use with npx (no installation required):

```bash
npx -y lsp-index-mcp /path/to/your/project
```

## Usage

### As MCP Server in Claude Desktop

Add to your Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "lsp-index": {
      "command": "npx",
      "args": [
        "-y",
        "lsp-index-mcp",
        "/path/to/your/project"
      ]
    }
  }
}
```

### With Environment Variable

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

## Supported Languages

- **Rust** (rust-analyzer) - `Cargo.toml`
- **Java** (JDT LS) - `pom.xml`, `build.gradle`, `build.gradle.kts`
- **Go** (gopls) - `go.mod`
- **TypeScript** - `package.json`

## Features

- `lsp_goto_definition` - Jump to symbol definition
- `lsp_find_references` - Find all references
- `lsp_hover` - Get type info and documentation
- `lsp_workspace_symbols` - Search symbols across workspace
- `lsp_get_diagnostics` - Get lint/type errors
- `lsp_hybrid_search` - Combined symbol + text search

## Manual Binary Download

If npm install fails, download manually from [GitHub Releases](https://github.com/youtiaoguagua/llm-lsp-index/releases).

## License

MIT
