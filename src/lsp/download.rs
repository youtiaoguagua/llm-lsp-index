//! LSP server download and management
//!
//! Handles automatic download of LSP servers from configured URLs,
//! with support for domestic (China) mirrors.

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// LSP download configuration
#[derive(Debug, Clone)]
pub struct LspDownloadConfig {
    /// Primary download URL
    pub primary_url: String,
    /// Domestic mirror URL (e.g., for China)
    pub mirror_url: Option<String>,
    /// Archive type
    pub archive_type: ArchiveType,
    /// Expected executable path within extracted archive
    pub executable_path: Vec<String>,
    /// Size hint for progress (optional)
    pub size_hint: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ArchiveType {
    TarGz,
    Zip,
}

/// LSP server download manager
pub struct LspDownloader {
    cache_dir: PathBuf,
    use_mirror: bool,
}

impl LspDownloader {
    /// Create new downloader
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Cannot find cache directory")?
            .join("lsp-index")
            .join("servers");

        // Check if should use mirror (based on env or latency test)
        let use_mirror = should_use_mirror();

        Ok(Self {
            cache_dir,
            use_mirror,
        })
    }

    /// Ensure an LSP server is downloaded and available
    pub async fn ensure_lsp(&self, name: &str, config: &LspDownloadConfig) -> Result<PathBuf> {
        let install_dir = self.cache_dir.join(name);

        // Check if already exists
        if let Some(executable) = self.find_executable(&install_dir, config) {
            tracing::info!("LSP {} already cached at {:?}", name, executable);
            return Ok(executable);
        }

        tracing::info!("Downloading LSP: {}", name);

        // Choose URL
        let url = if self.use_mirror {
            config.mirror_url.as_ref()
                .unwrap_or(&config.primary_url)
                .clone()
        } else {
            config.primary_url.clone()
        };

        // Create temp directory for extraction
        let temp_extract = std::env::temp_dir()
            .join(format!("lsp-index-extract-{}-{}", name, std::process::id()));
        tokio::fs::create_dir_all(&temp_extract).await?;

        self.download_and_extract(&url, &temp_extract, &config.archive_type)
            .await
            .with_context(|| format!("Failed to download LSP: {}", name))?;

        // Find the actual content directory (handle versioned folders like jdt-language-server-1.41.0-*/)
        let entries: Vec<_> = std::fs::read_dir(&temp_extract)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();

        // Move content to install_dir
        if entries.len() == 1 {
            // Single directory (e.g., jdt-language-server-1.41.0-202504291445/)
            let src = entries[0].path();
            if install_dir.exists() {
                tokio::fs::remove_dir_all(&install_dir).await?;
            }
            tokio::fs::rename(&src, &install_dir).await?;
        } else {
            // Multiple items, move all
            tokio::fs::create_dir_all(&install_dir).await?;
            for entry in std::fs::read_dir(&temp_extract)? {
                let entry = entry?;
                let src = entry.path();
                let dest = install_dir.join(entry.file_name());
                tokio::fs::rename(&src, &dest).await?;
            }
        }

        // Cleanup temp
        tokio::fs::remove_dir_all(&temp_extract).await.ok();

        // Find executable after extraction
        if let Some(executable) = self.find_executable(&install_dir, config) {
            tracing::info!("LSP {} installed at {:?}", name, executable);
            Ok(executable)
        } else {
            anyhow::bail!("Executable not found after extraction in {:?}", install_dir);
        }
    }

    /// Find executable in install directory based on config
    fn find_executable(&self, install_dir: &Path, config: &LspDownloadConfig) -> Option<PathBuf> {
        if config.executable_path.is_empty() {
            return None;
        }

        // For JDT LS: find the launcher jar in plugins directory
        if config.executable_path == vec!["plugins".to_string()] {
            return find_jdt_launcher(&install_dir.join("plugins"));
        }

        // Standard path construction
        let path = install_dir.join(config.executable_path.iter().collect::<PathBuf>());
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Download and extract archive to temp directory
    async fn download_and_extract(
        &self,
        url: &str,
        dest_dir: &Path,
        archive_type: &ArchiveType,
    ) -> Result<()> {
        // Create temp file for download
        let temp_file = std::env::temp_dir()
            .join(format!("lsp-index-download-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));

        // Download
        tracing::info!("Downloading from: {}", url);
        self.download_file(url, &temp_file).await?;

        // Create destination directory
        tokio::fs::create_dir_all(dest_dir).await?;

        // Extract
        tracing::info!("Extracting to: {:?}", dest_dir);
        match archive_type {
            ArchiveType::TarGz => self.extract_tar_gz(&temp_file, dest_dir).await?,
            ArchiveType::Zip => self.extract_zip(&temp_file, dest_dir).await?,
        }

        // Cleanup
        tokio::fs::remove_file(&temp_file).await.ok();

        Ok(())
    }

    /// Download file with progress
    async fn download_file(&self, url: &str, dest: &Path) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let response = client.get(url).send().await?;
        let status = response.status();

        if !status.is_success() {
            anyhow::bail!("HTTP {}: {}", status, url);
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(dest, &bytes).await?;

        tracing::info!("Downloaded {} bytes", bytes.len());
        Ok(())
    }

    /// Extract tar.gz archive
    async fn extract_tar_gz(&self, archive: &Path, dest: &Path) -> Result<()> {
        // Use blocking task for sync operations
        let archive = archive.to_owned();
        let dest = dest.to_owned();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive)?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(gz);
            tar.unpack(&dest)?;
            Ok::<_, anyhow::Error>(())
        }).await??;

        Ok(())
    }

    /// Extract zip archive
    async fn extract_zip(&self, archive: &Path, dest: &Path) -> Result<()> {
        let archive = archive.to_owned();
        let dest = dest.to_owned();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive)?;
            let mut zip = zip::ZipArchive::new(file)?;

            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)?;
                let entry_path = dest.join(entry.name());

                if entry.is_dir() {
                    std::fs::create_dir_all(&entry_path)?;
                } else {
                    if let Some(parent) = entry_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let mut out = std::fs::File::create(&entry_path)?;
                    std::io::copy(&mut entry, &mut out)?;
                }
            }
            Ok::<_, anyhow::Error>(())
        }).await??;

        Ok(())
    }

    /// Get cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

/// Determine if we should use domestic mirror
fn should_use_mirror() -> bool {
    // Check environment variable first
    if let Ok(val) = std::env::var("LSP_INDEX_USE_MIRROR") {
        return val == "1" || val.to_lowercase() == "true";
    }

    // Check for Chinese timezone/locale
    if let Ok(tz) = std::env::var("TZ") {
        if tz.contains("Asia/Shanghai") || tz.contains("Asia/Chongqing") {
            return true;
        }
    }

    // Check system locale
    #[cfg(windows)]
    {
        // Windows Chinese locales
        if let Ok(lang) = std::env::var("LANG") {
            if lang.starts_with("zh_CN") || lang.starts_with("zh_TW") {
                return true;
            }
        }
    }

    // Default: use latency test
    false
}

/// Get download config for known LSP servers
pub fn get_lsp_download_config(language: &str) -> Option<LspDownloadConfig> {
    match language {
        "java" => Some(LspDownloadConfig {
            primary_url: "https://download.eclipse.org/jdtls/snapshots/jdt-language-server-latest.tar.gz".to_string(),
            mirror_url: Some("https://mirrors.tuna.tsinghua.edu.cn/eclipse/jdtls/snapshots/jdt-language-server-latest.tar.gz".to_string()),
            archive_type: ArchiveType::TarGz,
            executable_path: vec!["plugins".to_string()],
            size_hint: Some(70_000_000), // ~70MB
        }),
        _ => None,
    }
}

/// Find JDT launcher jar in plugins directory
pub fn find_jdt_launcher(plugins_dir: &Path) -> Option<PathBuf> {
    if !plugins_dir.exists() {
        return None;
    }

    let entries = std::fs::read_dir(plugins_dir).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("org.eclipse.equinox.launcher") && name.ends_with(".jar") {
            return Some(entry.path());
        }
    }

    None
}
