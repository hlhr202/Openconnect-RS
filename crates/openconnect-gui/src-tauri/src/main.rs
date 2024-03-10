// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

#[tauri::command]
fn disconnect(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Disconnect).unwrap_or_default();
}

#[tauri::command]
fn destory(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Destory).unwrap_or_default();
}

fn main() {
    sudo::escalate_if_needed().unwrap();

    tauri::Builder::default()
        .setup(|app| {
            AppState::handle(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet, connect, disconnect, destory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
