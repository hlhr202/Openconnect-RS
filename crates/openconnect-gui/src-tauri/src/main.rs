// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod oidc;
mod state;
use state::AppState;

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
    server: String,
) -> anyhow::Result<(), String> {
    app_state
        .connect_with_oidc(&server)
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

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        sudo::escalate_if_needed().unwrap();
    }

    dotenvy::from_path(".env.local").unwrap();

    tauri::Builder::default()
        .setup(|app| {
            AppState::handle(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            connect,
            disconnect,
            trigger_state_retrieve,
            connect_with_oidc
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
