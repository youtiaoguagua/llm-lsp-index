# Implementation Plan: Ripgrep Hybrid Search

## Overview

Add ripgrep-based text search capability and create a hybrid search tool that combines LSP symbol search (semantic) with ripgrep content search (text). This addresses the limitation that `lsp_workspace_symbols` cannot search comments, TODOs, or arbitrary text content.

## Architecture Decisions

1. **Use `ignore` + `grep` crates** - `ignore` provides parallel directory traversal with .gitignore support (same library ripgrep uses). `grep` provides the regex search engine. Together they provide ripgrep-equivalent functionality with zero external dependencies.
2. **Hybrid search returns unified results** - Combines symbol results (with type info) + text matches (with context)
3. **Respect .gitignore automatically** - `ignore::Walk` handles this by default
4. **Parallel execution** - Run LSP and text searches concurrently for performance

## Dependency Graph

```
Search Request
    │
    ├── LSP workspace/symbol (semantic)
    │       │
    │       └── Symbol results with type info
    │
    └── ripgrep content search (text)
            │
            └── Text matches with line context
                    │
                    └── Combine & deduplicate
                            │
                            └── Hybrid results
```

## Task List

### Phase 1: Ripgrep Integration

#### Task 1: Add grep dependency and basic search
**Description:** Add the `grep` crate and implement basic file content search functionality.

**Acceptance Criteria:**
- [ ] Add `grep = "0.3"` and `grep-regex = "0.1"` to Cargo.toml
- [ ] Create `src/search/` module with mod.rs
- [ ] Implement `search_text()` function that searches files for a pattern
- [ ] Returns: file path, line number, matching line content, and surrounding context

**Verification:**
- [ ] `cargo build` succeeds
- [ ] Unit test: search "TODO" in test fixtures returns matches
- [ ] Respects .gitignore patterns

**Dependencies:** None
**Files:** `Cargo.toml`, `src/search/mod.rs`, `src/search/ripgrep.rs`
**Scope:** M

#### Task 2: Create unified search result type
**Description:** Define a common result structure that can represent both symbol hits and text matches.

**Acceptance Criteria:**
- [ ] Create `SearchResult` enum with variants: `Symbol` and `TextMatch`
- [ ] `Symbol` contains: name, kind (function/struct/etc), file, line, type info
- [ ] `TextMatch` contains: file, line, matched text, context lines
- [ ] Implement sorting by relevance (exact match > partial match)

**Verification:**
- [ ] Type compiles and serializes to JSON correctly
- [ ] Test: mixed list of Symbol and TextMatch sorts correctly

**Dependencies:** Task 1
**Files:** `src/search/mod.rs`
**Scope:** S

### Checkpoint: After Tasks 1-2
- [ ] Basic ripgrep search works
- [ ] Unified result type defined
- [ ] Unit tests pass

### Phase 2: Hybrid Search Tool

#### Task 3: Implement `lsp_hybrid_search` tool
**Description:** Create a new MCP tool that runs both workspace/symbol and ripgrep search concurrently, merging results.

**Acceptance Criteria:**
- [ ] Add tool definition to `McpTool::definitions()`
- [ ] Parameters: `query` (required), `include_text` (optional, default true), `include_symbols` (optional, default true)
- [ ] Run LSP workspace/symbol and ripgrep searches in parallel using `tokio::join!`
- [ ] Merge results: prioritize symbols, include text matches for lines not already covered by symbols
- [ ] Deduplicate: if a text match points to a symbol definition, keep only the symbol
- [ ] Return top 10 results by default

**Verification:**
- [ ] Test: search "User" returns both struct definition (symbol) and variable usages (text)
- [ ] Test: search "TODO" returns only text matches (no symbols)
- [ ] Test: search "test_function" returns symbol match, text matches are deduplicated

**Dependencies:** Task 1, Task 2
**Files:** `src/mcp/tools.rs`, `src/mcp/server.rs`, `src/search/mod.rs`
**Scope:** M

#### Task 4: Add code context extraction for text matches
**Description:** For text search results, extract surrounding code context similar to symbol results.

**Acceptance Criteria:**
- [ ] Reuse `extract_snippet()` from `src/bridge/snippet.rs`
- [ ] For each text match, show ±5 lines of context
- [ ] Format: `File: path:line\n[context]`

**Verification:**
- [ ] Test: text search shows code snippets, not just single lines

**Dependencies:** Task 3
**Files:** `src/search/mod.rs`
**Scope:** S

### Checkpoint: After Tasks 3-4
- [ ] `lsp_hybrid_search` tool works end-to-end
- [ ] Integration tests pass
- [ ] Results include both symbols and text matches

### Phase 3: Enhanced Search Features

#### Task 5: Add file type filtering
**Description:** Allow filtering search by file extension or language.

**Acceptance Criteria:**
- [ ] Add `file_types` parameter (e.g., `["rs", "toml"]`)
- [ ] ripgrep only searches specified file types
- [ ] LSP search unchanged (already language-specific)

**Verification:**
- [ ] Test: search with `file_types: ["md"]` only returns markdown matches

**Dependencies:** Task 3
**Files:** `src/mcp/tools.rs`, `src/search/ripgrep.rs`
**Scope:** S

#### Task 6: Add regex pattern support
**Description:** Support regex patterns for advanced text search.

**Acceptance Criteria:**
- [ ] Add `use_regex` parameter (default false)
- [ ] When true, treat query as regex pattern
- [ ] Validate regex and return error for invalid patterns

**Verification:**
- [ ] Test: regex search `TODO|FIXME|XXX` finds all three patterns
- [ ] Test: invalid regex returns helpful error

**Dependencies:** Task 1
**Files:** `src/search/ripgrep.rs`
**Scope:** S

### Checkpoint: Complete
- [ ] All search features implemented
- [ ] Documentation updated
- [ ] Ready for review

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| grep crate performance on large codebases | Med | Benchmark on large repo, consider adding timeout |
| Too many text results overwhelm user | Med | Limit to top 10, add pagination parameter |
| Regex injection attacks | Low | Validate regex before execution, use safe defaults |
| Binary file handling | Low | ripgrep automatically skips binary files |

## API Changes

### New Tool: `lsp_hybrid_search`

```json
{
  "name": "lsp_hybrid_search",
  "description": "Hybrid search combining LSP symbols and text search. Finds code symbols, comments, TODOs, and arbitrary text.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query (supports regex if use_regex=true)"
      },
      "include_symbols": {
        "type": "boolean",
        "description": "Include LSP symbol results",
        "default": true
      },
      "include_text": {
        "type": "boolean",
        "description": "Include text search results",
        "default": true
      },
      "file_types": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Filter by file extensions (e.g., ['rs', 'toml'])"
      },
      "use_regex": {
        "type": "boolean",
        "description": "Treat query as regex pattern",
        "default": false
      },
      "max_results": {
        "type": "integer",
        "description": "Maximum results to return",
        "default": 10
      }
    },
    "required": ["query"]
  }
}
```

## Open Questions

1. Should we cache search results for repeated queries?
2. Should we add a separate `lsp_text_search` tool for pure text search?
3. How to handle very large result sets (1000+ matches)?
