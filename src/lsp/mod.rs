//! LSP client module - headless LSP process management

mod registry;
mod process;
mod client;
pub mod watcher;
mod types;

pub use registry::LspRegistry;
pub use client::LspClient;
pub use process::LspProcess;
pub use watcher::{FileWatcher, FileChangeEvent, FileChangeKind};