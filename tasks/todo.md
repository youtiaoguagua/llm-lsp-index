# Task List: Ripgrep Hybrid Search

## Phase 1: Ripgrep Integration

- [ ] Task 1: Add grep dependency and basic search
  - Add `grep` and `grep-regex` to Cargo.toml
  - Create `src/search/mod.rs` and `src/search/ripgrep.rs`
  - Implement `search_text()` function
  - Write unit tests

- [ ] Task 2: Create unified search result type
  - Define `SearchResult` enum (Symbol | TextMatch)
  - Implement sorting by relevance
  - Write tests for mixed result sorting

**Checkpoint 1:** Basic ripgrep works, unified type defined

## Phase 2: Hybrid Search Tool

- [ ] Task 3: Implement `lsp_hybrid_search` tool
  - Add tool definition to `McpTool::definitions()`
  - Implement parallel LSP + ripgrep search
  - Merge and deduplicate results
  - Add handler in `execute_tool()`

- [ ] Task 4: Add code context extraction for text matches
  - Reuse `extract_snippet()` for text matches
  - Show ±5 lines context
  - Format results consistently

**Checkpoint 2:** Hybrid search works end-to-end

## Phase 3: Enhanced Search Features

- [ ] Task 5: Add file type filtering
  - Add `file_types` parameter
  - Filter ripgrep by file extension

- [ ] Task 6: Add regex pattern support
  - Add `use_regex` parameter
  - Validate regex patterns
  - Return helpful errors for invalid regex

**Checkpoint 3:** All features complete, ready for review

## Dependencies

```
Task 1 → Task 2 → Task 3 → Task 4
  ↓              ↓
Task 6          Task 5
```

## Verification Checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] Manual test: `lsp_hybrid_search` returns symbols + text
- [ ] Manual test: search "TODO" finds comments
- [ ] Manual test: regex search works
- [ ] File type filtering works
