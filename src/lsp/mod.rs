//! LSP client module - headless LSP process management

mod registry;
mod process;
mod client;
pub mod watcher;
mod types;
pub mod java_virtual_uri;

pub use registry::{LspRegistry, LspConfig};
pub use client::LspClient;
pub use process::LspProcess;
pub use watcher::{FileWatcher, FileChangeEvent, FileChangeKind};
pub use java_virtual_uri::JavaVirtualUriHandler;