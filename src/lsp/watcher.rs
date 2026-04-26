//! File watcher - monitors workspace for file changes
//!
//! Uses notify crate to watch for file changes and sync them to LSP via didChange notifications.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

/// File watcher for workspace changes
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    workspace_root: std::path::PathBuf,
}

/// File change event for LSP sync
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    pub path: std::path::PathBuf,
    pub kind: FileChangeKind,
}

#[derive(Debug, Clone)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
}

impl FileWatcher {
    /// Create a new file watcher with async event channel
    pub fn new(
        workspace_root: &Path,
        event_tx: mpsc::UnboundedSender<FileChangeEvent>,
    ) -> Result<Self, notify::Error> {
        let workspace_root = workspace_root.to_path_buf();

        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    Self::handle_event(event, &event_tx);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&workspace_root, RecursiveMode::Recursive)?;

        tracing::info!("File watcher started for: {}", workspace_root.display());

        Ok(Self {
            watcher,
            workspace_root,
        })
    }

    /// Handle notify events and convert to FileChangeEvent
    fn handle_event(
        event: Event,
        tx: &mpsc::UnboundedSender<FileChangeEvent>,
    ) {
        let kind = match event.kind {
            notify::EventKind::Create(_) => FileChangeKind::Created,
            notify::EventKind::Modify(_) => FileChangeKind::Modified,
            notify::EventKind::Remove(_) => FileChangeKind::Deleted,
            _ => return, // Ignore other events
        };

        for path in event.paths {
            // Only watch Rust files for now
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let change_event = FileChangeEvent {
                    path,
                    kind: kind.clone(),
                };
                if tx.send(change_event).is_err() {
                    tracing::debug!("File watcher receiver dropped");
                }
            }
        }
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher.unwatch(&self.workspace_root).ok();
        tracing::info!("File watcher stopped");
    }
}

/// Spawn a task to process file changes and send didChange to LSP
pub fn spawn_change_handler(
    mut rx: mpsc::UnboundedReceiver<FileChangeEvent>,
    client: std::sync::Arc<tokio::sync::Mutex<crate::lsp::client::LspClient>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            tracing::debug!("File change detected: {:?} - {:?}", event.path, event.kind);

            // Send didChange notification to LSP
            // For now, just log - full implementation would read file and send content
            let path_str = event.path.to_string_lossy();
            match event.kind {
                FileChangeKind::Created | FileChangeKind::Modified => {
                    if let Err(e) =
                        client.lock().await.send_did_change(&path_str, None).await
                    {
                        tracing::error!("Failed to send didChange: {}", e);
                    }
                }
                FileChangeKind::Deleted => {
                    if let Err(e) = client.lock().await.send_did_close(&path_str).await {
                        tracing::error!("Failed to send didClose: {}", e);
                    }
                }
            }
        }
    })
}
