// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod command;
mod state;
mod system_tray;

use command::*;
use openconnect_core::storage::StoredConfigs;
use state::AppState;
use system_tray::AppSystemTray;
use tauri::{Emitter, Manager, path::BaseDirectory};

fn main() {
    let config_file = StoredConfigs::getorinit_config_file().expect("failed to get config file");

    #[cfg(target_os = "linux")]
    {
        // TODO: add support for Polkit/pkexec GUI escalation
        sudo::escalate_if_needed().unwrap();
    }

    #[cfg(target_os = "macos")]
    {
        #[cfg(debug_assertions)]
        sudo::escalate_if_needed().unwrap();

        use openconnect_core::elevator::macos::{elevate, is_elevated};
        let exe_path = std::env::current_exe().expect("failed to get current executable path");
        let exe_path = exe_path
            .to_str()
            .expect("failed to convert exec path to string");

        let command = std::process::Command::new(exe_path);

        if !is_elevated() {
            elevate(&command).unwrap();
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "windows")]
    {
        use openconnect_core::elevator::windows::{elevate, is_elevated};
        let exe_path = std::env::current_exe().expect("failed to get current executable path");
        let exe_path = exe_path
            .to_str()
            .expect("failed to convert exec path to string");
        let args = std::env::args().skip(1);
        let mut command = std::process::Command::new(exe_path);
        let command = command.args(args);

        if !is_elevated() {
            #[cfg(debug_assertions)]
            const IS_DEBUG: bool = true;

            #[cfg(not(debug_assertions))]
            const IS_DEBUG: bool = false;

            elevate(command, IS_DEBUG).unwrap();
            std::process::exit(0);
        }
    }

    let app_system_tray = AppSystemTray::new();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .register_uri_scheme_protocol("oidcvpn", |ctx, req| {
            let app = ctx.app_handle();
            let uri = req.uri().to_string();

            // Emit a structured JSON event instead of `window.eval(...)` to prevent JS injection.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("oidcvpn-uri", uri);
            }

            tauri::http::Response::builder()
                .header("Content-Type", "text/plain; charset=utf-8")
                .status(200)
                .body(b"Authenticated, close this window and return to the application.".to_vec())
                .expect("valid static HTTP response")
        })
        .setup(move |app| {
            openconnect_core::log::Logger::init().expect("failed to init logger");

            let vpnc_script = {
                #[cfg(target_os = "windows")]
                {
                    let resource_path = app
                        .path()
                        .resolve("vpnc-script-win.js", BaseDirectory::Resource)
                        .expect("failed to resolve vpnc-script-win.js resource");

                    dunce::canonicalize(resource_path)
                        .expect("failed to canonicalize path")
                        .to_string_lossy()
                        .to_string()
                }

                #[cfg(not(target_os = "windows"))]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let resource_path = app
                        .path()
                        .resolve("vpnc-script", BaseDirectory::Resource)
                        .expect("failed to resolve vpnc-script resource");

                    let file = std::fs::OpenOptions::new()
                        .write(false)
                        .create(false)
                        .append(false)
                        .read(true)
                        .open(&resource_path)
                        .expect("failed to open vpnc-script");

                    let permissions = file.metadata()?.permissions();
                    if permissions.mode() & 0o111 == 0 {
                        let mut permissions = permissions;
                        permissions.set_mode(0o755);
                        file.set_permissions(permissions)?;
                    }

                    resource_path.to_string_lossy().to_string()
                }
            };

            let app_handle = app.handle().clone();
            app_system_tray.init_tray(&app_handle)?;

            tauri::async_runtime::block_on(async {
                app_handle.manage(app_system_tray.clone());
                AppState::handle_with_vpnc_script(app, &vpnc_script, config_file).await?;
                app_system_tray.recreate(&app_handle).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            disconnect,
            trigger_state_retrieve,
            get_stored_configs,
            upsert_stored_server,
            set_default_server,
            remove_server,
            connect_with_password,
            connect_with_oidc,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
            && let Some(win) = app.get_webview_window(label.as_str())
        {
            let _ = win.hide();
            api.prevent_close();
        }
    });
}
