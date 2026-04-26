//! LSP Index GUI - Tauri Desktop Application

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex;

/// LSP Manager state shared across commands
struct LspManager {
    /// Active LSP clients
    clients: Arc<Mutex<Vec<LspClientInfo>>>,
}

/// Information about an LSP client
#[derive(serde::Serialize, Clone)]
struct LspClientInfo {
    language: String,
    status: String,
    workspace: String,
}

/// Workspace info
#[derive(serde::Serialize)]
struct WorkspaceInfo {
    path: String,
    detected_language: Option<String>,
}

#[tauri::command]
async fn detect_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let workspace_path = PathBuf::from(&path);

    // Use our existing registry to detect language
    let registry = lsp_index::lsp::LspRegistry::new();
    let detected = registry.detect_language(&workspace_path);

    Ok(WorkspaceInfo {
        path: path.clone(),
        detected_language: detected.map(|c| c.language.clone()),
    })
}

#[tauri::command]
async fn start_lsp(language: String, workspace: String) -> Result<LspClientInfo, String> {
    // This would integrate with our LspClient
    // For now, return mock info
    Ok(LspClientInfo {
        language,
        status: "running".to_string(),
        workspace,
    })
}

#[tauri::command]
async fn list_active_lsps(state: State<'_, LspManager>) -> Result<Vec<LspClientInfo>, String> {
    let clients = state.clients.lock().await;
    Ok(clients.clone())
}

#[tauri::command]
async fn stop_lsp(language: String, state: State<'_, LspManager>) -> Result<(), String> {
    let mut clients = state.clients.lock().await;
    clients.retain(|c| c.language != language);
    Ok(())
}

#[tauri::command]
async fn goto_definition(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
) -> Result<String, String> {
    // This would call our LSP client
    // For now, return a placeholder
    Ok(format!("Goto definition for {} at {}:{}:{}", language, file_path, line, character))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(LspManager {
            clients: Arc::new(Mutex::new(Vec::new())),
        })
        .setup(|app| {
            // Setup logging
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_workspace,
            start_lsp,
            list_active_lsps,
            stop_lsp,
            goto_definition
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
