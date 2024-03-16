// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod oidc;
mod state;

use openconnect_core::storage::{StoredConfigs, StoredConfigsJson, StoredServer};
use state::AppState;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
async fn connect_with_password(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> anyhow::Result<(), String> {
    app_state
        .connect_with_user_pass(&server_name)
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

#[tauri::command]
async fn upsert_stored_server(server: StoredServer) -> anyhow::Result<(), String> {
    let mut stored_configs = StoredConfigs::new();
    stored_configs
        .read_from_file()
        .await
        .map_err(|e| e.to_string())?;
    stored_configs
        .upsert_server(server)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        sudo::escalate_if_needed().unwrap();
    }

    #[cfg(target_os = "macos")]
    {
        #[cfg(debug_assertions)]
        sudo::escalate_if_needed().unwrap();

        unsafe {
            if libc::geteuid() != 0 && openconnect_core::helper_reluanch_as_root() == 1 {
                std::process::exit(0);
            }
        }
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
            let vpnc_script = {
                #[cfg(target_os = "windows")]
                {
                    let resource_path = app
                        .path_resolver()
                        .resolve_resource("vpnc-script-win.js")
                        .expect("failed to resolve resource");

                    dunce::canonicalize(resource_path)
                        .expect("failed to canonicalize path")
                        .to_string_lossy()
                        .to_string()
                }

                #[cfg(not(target_os = "windows"))]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let resource_path = app
                        .path_resolver()
                        .resolve_resource("vpnc-script")
                        .expect("failed to resolve resource");

                    let file = std::fs::OpenOptions::new()
                        .write(false)
                        .create(false)
                        .append(false)
                        .read(true)
                        .open(resource_path)
                        .expect("failed to open file");

                    let permissions = file.metadata().unwrap().permissions();
                    let is_executable = permissions.mode() & 0o111 != 0;
                    if !is_executable {
                        let mut permissions = permissions;
                        permissions.set_mode(0o755);
                        file.set_permissions(permissions).unwrap();
                    }

                    resource_path.to_string_lossy().to_string()
                }
            };

            AppState::handle_with_vpnc_script(app, &vpnc_script);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            disconnect,
            trigger_state_retrieve,
            get_stored_configs,
            upsert_stored_server,
            connect_with_password,
            connect_with_oidc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
