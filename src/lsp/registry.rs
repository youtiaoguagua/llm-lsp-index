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
    /// Whether this LSP uses virtual URIs (e.g., jdt:// for Java)
    pub supports_virtual_uris: bool,
}

impl LspConfig {
    /// Rust language configuration
    pub fn rust() -> Self {
        Self {
            language: "rust".to_string(),
            binary_name: "rustup".to_string(),
            binary_path: None,
            root_file: "Cargo.toml".to_string(),
            supports_virtual_uris: false,
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
            "java" => {
                // JDT LS requires special spawn command
                // Try environment variable first, then common locations
                let jdt_ls_path = std::env::var("JDT_LS_PATH")
                    .or_else(|_| {
                        // Common installation paths
                        let paths = [
                            "/usr/share/java/jdtls/plugins/org.eclipse.equinox.launcher_*.jar",
                            "/opt/jdtls/plugins/org.eclipse.equinox.launcher_*.jar",
                            "~/jdtls/plugins/org.eclipse.equinox.launcher_*.jar",
                        ];
                        for pattern in &paths {
                            if let Ok(entries) = glob::glob(pattern) {
                                for entry in entries.flatten() {
                                    return Ok(entry.to_string_lossy().to_string());
                                }
                            }
                        }
                        Err("JDT LS not found")
                    })
                    .unwrap_or_else(|_| "jdt-language-server".to_string());

                vec![
                    "java".to_string(),
                    "-jar".to_string(),
                    jdt_ls_path,
                ]
            }
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
            supports_virtual_uris: false,
        }
    }

    /// TypeScript language configuration
    pub fn typescript() -> Self {
        Self {
            language: "typescript".to_string(),
            binary_name: "typescript-language-server".to_string(),
            binary_path: None,
            root_file: "package.json".to_string(),
            supports_virtual_uris: false,
        }
    }

    /// Java language configuration
    pub fn java() -> Self {
        Self {
            language: "java".to_string(),
            // JDT LS is typically run via java -jar
            binary_name: "java".to_string(),
            binary_path: None,
            // Java projects can be identified by pom.xml (Maven) or build.gradle (Gradle)
            root_file: "pom.xml".to_string(),
            supports_virtual_uris: true,
        }
    }

    /// Java Gradle configuration (alternative root file)
    pub fn java_gradle() -> Self {
        Self {
            language: "java".to_string(),
            binary_name: "java".to_string(),
            binary_path: None,
            root_file: "build.gradle".to_string(),
            supports_virtual_uris: true,
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
                LspConfig::java(),
                LspConfig::java_gradle(),
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