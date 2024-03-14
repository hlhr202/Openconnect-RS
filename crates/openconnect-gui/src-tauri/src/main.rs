// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod oidc;
mod state;
use openconnect_core::storage::{StoredConfigs, StoredConfigsJson};
use state::AppState;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
async fn connect(
    app_state: tauri::State<'_, AppState>,
    server: String,
    username: String,
    password: String,
) -> anyhow::Result<(), String> {
    app_state
        .connect_with_user_pass(&server, &username, &password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn connect_with_oidc(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> anyhow::Result<(), String> {
    app_state
        .connect_with_oidc(&server_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn disconnect(app_state: tauri::State<'_, AppState>) -> anyhow::Result<(), String> {
    app_state.disconnect().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn trigger_state_retrieve(
    app_state: tauri::State<'_, AppState>,
) -> anyhow::Result<(), String> {
    app_state
        .trigger_state_retrieve()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_stored_configs() -> anyhow::Result<StoredConfigsJson, String> {
    let mut stored_configs = StoredConfigs::new();
    stored_configs
        .read_from_file()
        .await
        .map_err(|e| e.to_string())?;
    Ok(stored_configs.into())
}

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        sudo::escalate_if_needed().unwrap();
    }

    tauri::Builder::default()
        .register_uri_scheme_protocol("oidcvpn", |app, _req| {
            let _app_state: tauri::State<'_, AppState> = app.state();

            tauri::http::ResponseBuilder::new()
                .header("Content-Type", "text/html")
                .status(200)
                .body(b"Authenticated, close this window and return to the application.".to_vec())
        })
        .setup(|app| {
            AppState::handle(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            connect,
            disconnect,
            trigger_state_retrieve,
            connect_with_oidc,
            get_stored_configs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
