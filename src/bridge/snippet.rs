//! Code snippet extraction - read file sections around target positions

use std::fs;

/// Maximum lines to include in snippet (configurable)
const DEFAULT_SNIPPET_LINES: usize = 50;

/// Extract a code snippet from a source string around a target line
///
/// Similar to `extract_snippet` but works with source content directly
/// instead of reading from a file path. Used for virtual URIs (Java JDT LS).
pub fn extract_snippet_from_source(
    source: &str,
    target_line: u32,
    context_lines: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = source.lines().collect();
    let target_idx = target_line as usize;
    let mut start = target_idx.saturating_sub(context_lines);
    let mut end = std::cmp::min(target_idx + context_lines + 1, lines.len());

    // Ensure snippet doesn't exceed max lines
    let snippet_lines = end - start;
    if snippet_lines > DEFAULT_SNIPPET_LINES {
        let half = DEFAULT_SNIPPET_LINES / 2;
        start = target_idx.saturating_sub(half);
        end = std::cmp::min(target_idx + half + 1, lines.len());
    }

    let snippet = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>4}: {}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(snippet)
}

/// Extract a code snippet from a file around a target line
pub fn extract_snippet(
    file_path: &str,
    target_line: u32,
    context_lines: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    extract_snippet_from_source(&content, target_line, context_lines)
}

/// Extract multiple snippets (for multiple implementations)
pub fn extract_snippets(
    locations: &[(String, u32)],
    context_lines: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    locations
        .iter()
        .map(|(file, line)| extract_snippet(file, *line, context_lines))
        .collect()
}

/// Extract snippets from virtual URIs (for Java JDT LS)
///
/// Takes pre-fetched source content (from JDT LS decompilation)
/// and extracts the snippet around the target line.
pub fn extract_snippet_virtual(
    source: &str,
    virtual_path: &str,
    line: u32,
    context_lines: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let snippet = extract_snippet_from_source(source, line, context_lines)?;
    // Prepend virtual path info
    Ok(format!("// Virtual URI: {}\n{}\n// (decompiled/generated source)", virtual_path, snippet))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_snippet_from_source() {
        let source = r#"line 1
line 2
line 3
line 4
line 5
line 6
line 7
line 8
line 9
line 10"#;

        // Target line 4 (0-indexed = 3)
        let result = extract_snippet_from_source(source, 3, 2).unwrap();
        assert!(result.contains("2: line 2"));
        assert!(result.contains("3: line 3"));
        assert!(result.contains("4: line 4"));
        assert!(result.contains("5: line 5"));
        assert!(result.contains("6: line 6"));
        assert!(!result.contains("7: line 7")); // Outside context
        assert!(!result.contains("1: line 1")); // Outside context
    }

    #[test]
    fn test_extract_snippet_virtual() {
        let source = "public class Test {\n    public void method() {\n    }\n}";
        let result = extract_snippet_virtual(source, "jdt://contents/Test.class", 1, 5).unwrap();
        assert!(result.contains("Virtual URI: jdt://contents/Test.class"));
        assert!(result.contains("decompiled/generated source"));
        assert!(result.contains("public class Test"));
    }
}