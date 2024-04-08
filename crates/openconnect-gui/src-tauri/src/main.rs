// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod command;
mod oidc;
mod state;
mod system_tray;

use command::*;
use state::AppState;
use system_tray::AppSystemTray;
use tauri::Manager;

fn main() {
    #[cfg(target_os = "linux")]
    {
        // TODO: add support for GUI escalation
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
        // get command of current execution
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

    let app_system_tray_clone = app_system_tray.clone();
    let app = tauri::Builder::default()
        .system_tray(app_system_tray.create_empty())
        .on_system_tray_event(move |app, event| app_system_tray_clone.handle(app, event))
        .register_uri_scheme_protocol("oidcvpn", |app, _req| {
            println!("URI: {:?}", _req.uri());
            let _app_state: tauri::State<'_, AppState> = app.state();

            let window = app.get_window("main");
            if let Some(window) = window {
                let uri = _req.uri().to_string();
                let _ = window.eval(format!("window.alert('URI: {}')", uri).as_str());
            }

            tauri::http::ResponseBuilder::new()
                .header("Content-Type", "text/html")
                .status(200)
                .body(b"Authenticated, close this window and return to the application.".to_vec())
        })
        .setup(move |app| {
            openconnect_core::log::Logger::init().expect("failed to init logger");
            // This is to fully remove dock icon, temp disable
            // #[cfg(target_os = "macos")]
            // app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
                        .open(resource_path.clone())
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

            let window = app.get_window("main").expect("no main window");

            #[cfg(any(windows, target_os = "macos"))]
            window_shadows::set_shadow(&window, true).unwrap();

            let app_handle = app.app_handle();

            Ok(tauri::async_runtime::block_on(async {
                app_handle.manage(app_system_tray.clone());
                AppState::handle_with_vpnc_script(app, &vpnc_script)
                    .await
                    .unwrap();
                app_system_tray.recreate(&app_handle).await
            })?)
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
        .unwrap();

    app.run(|app, event| {
        if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            let win = app.get_window(label.as_str()).unwrap();
            win.hide().unwrap();
            api.prevent_close();
        }
    });
}
