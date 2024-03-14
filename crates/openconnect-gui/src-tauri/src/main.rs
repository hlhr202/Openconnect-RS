// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod oidc;
mod state;
use state::{AppState, VpnCommand};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
fn connect(app_state: tauri::State<AppState>, server: String, username: String, password: String) {
    app_state
        .send(VpnCommand::Connect {
            server,
            username,
            password,
        })
        .unwrap_or_default();
}

#[tauri::command(rename_all = "snake_case")]
fn connect_with_oidc(app_state: tauri::State<AppState>, server: String) {
    app_state
        .send(VpnCommand::ConnectOpenID { server })
        .unwrap_or_default();
}

#[tauri::command]
fn disconnect(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Disconnect).unwrap_or_default();
}

#[tauri::command]
fn destory(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Destory).unwrap_or_default();
}

#[tauri::command]
fn get_current_state(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::GetState).unwrap_or_default();
}

fn main() {
    #[cfg(not(target_os = "windows"))]
    sudo::escalate_if_needed().unwrap();
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
            destory,
            get_current_state,
            connect_with_oidc
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
