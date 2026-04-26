//! Code snippet extraction - read file sections around target positions

use std::fs;

/// Maximum lines to include in snippet (configurable)
const DEFAULT_SNIPPET_LINES: usize = 50;

/// Extract a code snippet from a file around a target line
pub fn extract_snippet(
    file_path: &str,
    target_line: u32,
    context_lines: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

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