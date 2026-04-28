//! LSP type utilities - helpers for LSP types handling
#![allow(dead_code)]

use lsp_types::*;

/// Convert file path to LSP URI
pub fn file_path_to_uri(path: &str) -> String {
    format!("file://{}", path)
}

/// Convert URI to file path
pub fn uri_to_file_path(uri: &str) -> Option<String> {
    uri.strip_prefix("file://").map(|s| s.to_string())
}

/// Create a Position from line and character
pub fn create_position(line: u32, character: u32) -> Position {
    Position { line, character }
}

/// Create a TextDocumentIdentifier from file path
pub fn create_text_document_identifier(file_path: &str) -> TextDocumentIdentifier {
    use url::Url;
    let uri = Url::parse(&file_path_to_uri(file_path)).unwrap();
    TextDocumentIdentifier { uri }
}

/// Create a TextDocumentPositionParams
pub fn create_text_document_position_params(
    file_path: &str,
    line: u32,
    character: u32,
) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: create_text_document_identifier(file_path),
        position: create_position(line, character),
    }
}