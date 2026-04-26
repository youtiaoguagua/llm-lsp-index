//! Text searcher implementation using ignore + grep crates

use grep::regex::RegexMatcher;
use grep::searcher::sinks::Lossy;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use ignore::Walk;

/// A text match result
#[derive(Debug, Clone)]
pub struct TextMatch {
    /// File path
    pub path: String,
    /// Line number (1-based)
    pub line_number: u64,
    /// Matched line content
    pub line: String,
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Root directory to search
    pub root: String,
    /// Maximum number of results
    pub max_results: usize,
    /// File type filters (e.g., ["rs", "toml"])
    pub file_types: Option<Vec<String>>,
    /// Case insensitive search
    pub case_insensitive: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            root: ".".to_string(),
            max_results: 100,
            file_types: None,
            case_insensitive: true,
        }
    }
}

/// Search for text pattern in files
///
/// Uses `ignore::Walk` for .gitignore-aware directory traversal
/// and `grep::searcher` for regex matching.
pub fn search_text(pattern: &str, options: &SearchOptions) -> Result<Vec<TextMatch>, Box<dyn std::error::Error>> {
    let mut matches = Vec::new();

    // Build regex matcher
    let pattern = if options.case_insensitive && !pattern.starts_with("(?i)") {
        format!("(?i){}", pattern)
    } else {
        pattern.to_string()
    };

    let matcher = RegexMatcher::new_line_matcher(&pattern)?;

    // Build searcher
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true)
        .build();

    // Walk directory
    let walker = Walk::new(&options.root);

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Apply file type filter
        if let Some(ref types) = options.file_types {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_string();
                if !types.contains(&ext) {
                    continue;
                }
            } else {
                continue;
            }
        }

        // Create a closure-based sink
        let path_str = path.to_string_lossy().to_string();
        let mut sink = Lossy(|line_number: u64, line: &str| {
            matches.push(TextMatch {
                path: path_str.clone(),
                line_number,
                line: line.trim_end().to_string(),
            });

            // Check if we've reached max results
            if matches.len() >= options.max_results {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Max results reached"
                ));
            }

            Ok(true)
        });

        if let Err(e) = searcher.search_path(&matcher, path, &mut sink) {
            // Check if it's our "max results" signal
            if e.to_string().contains("Max results reached") {
                return Ok(matches);
            }
            // Otherwise ignore errors for individual files
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_text_basic() {
        let options = SearchOptions {
            root: "tests/fixtures/rust-sample".to_string(),
            max_results: 10,
            ..Default::default()
        };

        let results = search_text("greet", &options).unwrap();
        assert!(!results.is_empty(), "Should find 'greet' in test fixtures");

        let first = &results[0];
        assert!(first.line.contains("greet"));
        assert!(first.line_number > 0);
    }

    #[test]
    fn test_search_text_case_insensitive() {
        let options = SearchOptions {
            root: "tests/fixtures/rust-sample".to_string(),
            max_results: 10,
            case_insensitive: true,
            ..Default::default()
        };

        // Search for lowercase, should find uppercase
        let results = search_text("GREET", &options).unwrap();
        assert!(!results.is_empty(), "Should find case-insensitive match");
    }
}
