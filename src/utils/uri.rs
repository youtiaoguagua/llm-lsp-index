//! URI utilities

/// Normalize a file URI to a consistent format
pub fn normalize_uri(uri: &str) -> String {
    // Handle Windows paths
    if uri.starts_with("file:///") {
        uri.replace("file:///", "file://")
    } else if !uri.starts_with("file://") {
        format!("file://{}", uri)
    } else {
        uri.to_string()
    }
}

/// Convert URI to local file path
pub fn uri_to_path(uri: &str) -> Option<String> {
    uri.strip_prefix("file://")
        .or_else(|| uri.strip_prefix("file:///"))
        .map(|s| s.to_string())
}