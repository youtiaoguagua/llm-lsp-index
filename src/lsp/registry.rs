//! Language detection and LSP configuration registry
//!
//! Maps project root files (Cargo.toml, go.mod, package.json) to corresponding LSP servers.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),
    #[error("LSP binary not found: {0}")]
    BinaryNotFound(String),
}

/// Supported languages and their LSP configurations
#[derive(Debug, Clone)]
pub struct LspConfig {
    pub language: String,
    pub binary_name: String,
    pub binary_path: Option<String>,
    pub root_file: String,
}

impl LspConfig {
    /// Rust language configuration
    pub fn rust() -> Self {
        Self {
            language: "rust".to_string(),
            binary_name: "rustup".to_string(),
            binary_path: None,
            root_file: "Cargo.toml".to_string(),
        }
    }

    /// Get the actual command to run the LSP server
    pub fn get_spawn_command(&self) -> Vec<String> {
        match self.language.as_str() {
            "rust" => vec![
                "rustup".to_string(),
                "run".to_string(),
                "stable".to_string(),
                "rust-analyzer".to_string()
            ],
            _ => vec![self.binary_name.clone()],
        }
    }

    /// Go language configuration
    pub fn go() -> Self {
        Self {
            language: "go".to_string(),
            binary_name: "gopls".to_string(),
            binary_path: None,
            root_file: "go.mod".to_string(),
        }
    }

    /// TypeScript language configuration
    pub fn typescript() -> Self {
        Self {
            language: "typescript".to_string(),
            binary_name: "typescript-language-server".to_string(),
            binary_path: None,
            root_file: "package.json".to_string(),
        }
    }
}

/// Registry for detecting language and matching LSP server
pub struct LspRegistry {
    configs: Vec<LspConfig>,
}

impl Default for LspRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LspRegistry {
    pub fn new() -> Self {
        Self {
            configs: vec![
                LspConfig::rust(),
                LspConfig::go(),
                LspConfig::typescript(),
            ],
        }
    }

    /// Detect language from project root directory
    pub fn detect_language(&self, root_path: &Path) -> Option<&LspConfig> {
        for config in &self.configs {
            let root_file = root_path.join(&config.root_file);
            tracing::debug!("Checking for root file: {:?}", root_file);
            if root_file.exists() {
                tracing::info!("Found language config: {} at {:?}", config.language, root_file);
                return Some(config);
            }
        }
        tracing::warn!("No language detected in {:?}", root_path);
        None
    }

    /// Get LSP config by language name
    pub fn get_config(&self, language: &str) -> Option<&LspConfig> {
        self.configs.iter().find(|c| c.language == language)
    }
}