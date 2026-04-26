//! File utilities

use std::fs;

/// Read specific lines from a file
pub fn read_file_lines(file_path: &str, start: usize, end: usize) -> Result<Vec<String>, std::io::Error> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    let start = std::cmp::min(start, lines.len());
    let end = std::cmp::min(end, lines.len());

    Ok(lines[start..end].iter().map(|s| s.to_string()).collect())
}

/// Check if file exists
pub fn file_exists(file_path: &str) -> bool {
    std::path::Path::new(file_path).exists()
}