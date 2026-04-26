//! Hybrid search - combines LSP symbol search with text search

use crate::lsp::LspClient;
use crate::search::{search_text, SearchOptions, TextMatch};
use serde::{Deserialize, Serialize};

/// Result from hybrid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub symbol_results: Vec<SymbolResult>,
    pub text_results: Vec<TextResult>,
}

/// Symbol result from LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolResult {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub description: Option<String>,
}

/// Text result from ripgrep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResult {
    pub file: String,
    pub line: u32,
    pub content: String,
}

/// Unified result type for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result_type")]
pub enum SearchResult {
    Symbol { name: String, kind: String, location: String },
    TextMatch { content: String, location: String },
}

/// Hybrid search options
#[derive(Debug, Clone)]
pub struct HybridSearchOptions {
    pub query: String,
    pub include_symbols: bool,
    pub include_text: bool,
    pub file_types: Option<Vec<String>>,
    pub max_results: usize,
}

impl Default for HybridSearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            include_symbols: true,
            include_text: true,
            file_types: None,
            max_results: 10,
        }
    }
}

/// Perform hybrid search combining LSP symbols and text search
///
/// This function runs both searches concurrently and merges the results.
/// Symbol results are prioritized over text matches.
pub async fn hybrid_search(
    client: Option<&mut LspClient>,
    workspace_root: &str,
    options: &HybridSearchOptions,
) -> Result<HybridResult, Box<dyn std::error::Error>> {
    let mut symbol_results = Vec::new();
    let mut text_results = Vec::new();

    // Run symbol search if enabled and client is available
    if options.include_symbols {
        if let Some(client) = client {
            match client.workspace_symbols(&options.query).await {
                Ok(result) => {
                    symbol_results = parse_symbol_results(result);
                }
                Err(e) => {
                    tracing::warn!("LSP symbol search failed: {}", e);
                }
            }
        }
    }

    // Run text search if enabled
    if options.include_text {
        let text_options = SearchOptions {
            root: workspace_root.to_string(),
            max_results: options.max_results * 2, // Get more to allow for deduplication
            file_types: options.file_types.clone(),
            case_insensitive: true,
        };

        match search_text(&options.query, &text_options) {
            Ok(matches) => {
                text_results = deduplicate_text_matches(matches, &symbol_results);
            }
            Err(e) => {
                tracing::warn!("Text search failed: {}", e);
            }
        }
    }

    // Limit total results
    symbol_results.truncate(options.max_results);
    let remaining = options.max_results.saturating_sub(symbol_results.len());
    text_results.truncate(remaining);

    Ok(HybridResult {
        symbol_results,
        text_results,
    })
}

/// Parse LSP workspace/symbol response
fn parse_symbol_results(result: serde_json::Value) -> Vec<SymbolResult> {
    let mut symbols = Vec::new();

    if let Some(arr) = result.as_array() {
        for sym in arr {
            let name = sym.get("name").and_then(|n| n.as_str()).unwrap_or("unknown").to_string();

            let kind_num = sym.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
            let kind = symbol_kind_to_string(kind_num);

            let location = sym.get("location");
            let file = location
                .and_then(|l| l.get("uri"))
                .and_then(|u| u.as_str())
                .map(|u| crate::mcp::server::McpServer::uri_to_path(u).unwrap_or(u.to_string()))
                .unwrap_or_default();

            let line = location
                .and_then(|l| l.get("range"))
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(|l| l.as_u64())
                .unwrap_or(0) as u32;

            let description = sym.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string());

            symbols.push(SymbolResult {
                name,
                kind,
                file,
                line,
                description,
            });
        }
    }

    symbols
}

/// Convert LSP symbol kind number to string
fn symbol_kind_to_string(kind: u64) -> String {
    match kind {
        1 => "File",
        2 => "Module",
        3 => "Namespace",
        4 => "Package",
        5 => "Class",
        6 => "Method",
        7 => "Property",
        8 => "Field",
        9 => "Constructor",
        10 => "Enum",
        11 => "Interface",
        12 => "Function",
        13 => "Variable",
        14 => "Constant",
        15 => "String",
        16 => "Number",
        17 => "Boolean",
        18 => "Array",
        19 => "Object",
        20 => "Key",
        21 => "Null",
        22 => "Struct",
        23 => "EnumMember",
        24 => "Event",
        25 => "Operator",
        26 => "TypeParameter",
        _ => "Symbol",
    }
    .to_string()
}

/// Remove text matches that point to already-found symbols
fn deduplicate_text_matches(
    matches: Vec<TextMatch>,
    symbols: &[SymbolResult],
) -> Vec<TextResult> {
    matches
        .into_iter()
        .filter_map(|m| {
            // Check if this line is already covered by a symbol
            let is_duplicate = symbols.iter().any(|s| {
                s.file == m.path && s.line == m.line_number as u32
            });

            if is_duplicate {
                None
            } else {
                Some(TextResult {
                    file: m.path,
                    line: m.line_number as u32,
                    content: m.line,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_to_string() {
        assert_eq!(symbol_kind_to_string(5), "Class");
        assert_eq!(symbol_kind_to_string(12), "Function");
        assert_eq!(symbol_kind_to_string(99), "Symbol");
    }

    #[test]
    fn test_deduplicate_text_matches() {
        let symbols = vec![
            SymbolResult {
                name: "test_fn".to_string(),
                kind: "Function".to_string(),
                file: "src/lib.rs".to_string(),
                line: 10,
                description: None,
            },
        ];

        let matches = vec![
            TextMatch {
                path: "src/lib.rs".to_string(),
                line_number: 10, // Same as symbol - should be deduplicated
                line: "fn test_fn()".to_string(),
            },
            TextMatch {
                path: "src/lib.rs".to_string(),
                line_number: 20, // Different line - should be kept
                line: "let x = 5;".to_string(),
            },
        ];

        let result = deduplicate_text_matches(matches, &symbols);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line, 20);
    }
}
