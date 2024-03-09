// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
use app_state::{AppState, VpnCommand};
use openconnect_core::Status;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
fn connect(app_state: tauri::State<AppState>, server: String, username: String, password: String) {
    app_state.send(VpnCommand::Connect {
        server,
        username,
        password,
    });
}

#[tauri::command]
fn disconnect(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Disconnect);
}

#[tauri::command]
fn destory(app_state: tauri::State<AppState>) {
    app_state.send(VpnCommand::Destory);
}

#[derive(Debug, Clone, Copy)]
pub enum VpnEvent {
    Status(Status),
}

fn main() {
    sudo::escalate_if_needed().unwrap();

    tauri::Builder::default()
        .setup(|app| {
            let (event_tx, event_rx) = std::sync::mpsc::channel::<VpnEvent>();
            let (app_state, command_rx) = AppState::new(event_tx);
            app_state.run(command_rx);
            app.manage(app_state);

            let handle = app.app_handle();

            std::thread::spawn(move || loop {
                if let Ok(event) = event_rx.recv() {
                    match event {
                        VpnEvent::Status(status) => {
                            let result = handle.emit_all("vpnStatus", Some(String::from(status)));
                            if let Err(e) = result {
                                eprintln!("Error while emitting event: {:?}", e);
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet, connect, disconnect, destory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
