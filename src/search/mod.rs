//! Text search module using ignore + grep crates
//!
//! Provides ripgrep-equivalent functionality:
//! - `ignore::Walk` for parallel directory traversal with .gitignore support
//! - `grep::searcher` for regex search engine

use serde::{Deserialize, Serialize};

pub mod searcher;
pub mod hybrid;

pub use searcher::{search_text, TextMatch, SearchOptions};
pub use hybrid::{hybrid_search, HybridResult, HybridSearchOptions, SearchResult};

/// Unified search result that can represent symbols or text matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum UnifiedResult {
    /// LSP Symbol result
    Symbol(SymbolResult),
    /// Text search result
    Text(TextResult),
}

/// LSP Symbol search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolResult {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub description: Option<String>,
}

/// Text search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResult {
    pub file: String,
    pub line: u32,
    pub content: String,
    pub context: String, // Surrounding lines for context
}

impl UnifiedResult {
    /// Get file path from any result type
    pub fn file(&self) -> &str {
        match self {
            UnifiedResult::Symbol(s) => &s.file,
            UnifiedResult::Text(t) => &t.file,
        }
    }

    /// Get line number from any result type
    pub fn line(&self) -> u32 {
        match self {
            UnifiedResult::Symbol(s) => s.line,
            UnifiedResult::Text(t) => t.line,
        }
    }

    /// Get display text for the result
    pub fn display(&self) -> String {
        match self {
            UnifiedResult::Symbol(s) => {
                format!("{} ({}) @ {}:{}", s.name, s.kind, s.file, s.line)
            }
            UnifiedResult::Text(t) => {
                format!("{} @ {}:{}", t.content.trim(), t.file, t.line)
            }
        }
    }
}
