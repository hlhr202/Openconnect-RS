use crate::state::AppState;
use openconnect_core::storage::{OidcServer, PasswordServer, StoredServer};
use std::sync::Arc;
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTraySubmenu,
};

pub struct AppSystemTray {
    // pub system_tray: SystemTray,
}

impl AppSystemTray {
    pub fn new() -> Arc<AppSystemTray> {
        Arc::new(AppSystemTray {})
    }

    pub async fn recreate(&self, app_handle: &AppHandle) -> Result<(), tauri::Error> {
        let app_state: State<'_, AppState> = app_handle.state();
        let stored_configs = app_state.stored_configs.read().await;
        let servers = stored_configs.servers.values().collect::<Vec<_>>();
        let (status, current_server_name) = app_state.get_status_and_name().await.unwrap();

        if status.status == "CONNECTED" {
            app_handle
                .tray_handle()
                .set_icon(tauri::Icon::Raw(
                    include_bytes!("../icons/connected-w.png").to_vec(),
                ))
                .unwrap();
        } else {
            app_handle
                .tray_handle()
                .set_icon(tauri::Icon::Raw(
                    include_bytes!("../icons/disconnected-w.png").to_vec(),
                ))
                .unwrap();
        }

        let server_menus = servers
            .iter()
            .map(|server| {
                let server_name = match server {
                    StoredServer::Oidc(OidcServer { name, .. }) => name.clone(),
                    StoredServer::Password(PasswordServer { name, .. }) => name.clone(),
                };
                let is_server_connected = current_server_name.as_ref() == Some(&server_name)
                    && status.status == "CONNECTED";

                if is_server_connected {
                    CustomMenuItem::new(
                        format!("disconnect-{}", server_name),
                        format!("Disconnect {}", server_name),
                    )
                } else {
                    let conection_menu = CustomMenuItem::new(
                        format!("connect-{}", server_name),
                        format!("Connect {}", server_name),
                    );

                    if status.status == "CONNECTED" {
                        conection_menu.disabled()
                    } else {
                        conection_menu
                    }
                }
            })
            .collect::<Vec<_>>();

        let mut sub_menu_tray = SystemTrayMenu::new();

        for server_menu in server_menus {
            sub_menu_tray = sub_menu_tray.add_item(server_menu);
        }

        let sub_menu = SystemTraySubmenu::new("Servers", sub_menu_tray);
        let show = CustomMenuItem::new("show".to_string(), "Show Window");
        let quit = CustomMenuItem::new("quit".to_string(), "Quit");

        let tray_menu = SystemTrayMenu::new()
            .add_submenu(sub_menu)
            .add_native_item(tauri::SystemTrayMenuItem::Separator)
            .add_item(show)
            .add_item(quit);

        let system_tray_handle = app_handle.tray_handle();
        system_tray_handle.set_menu(tray_menu)
    }

    pub fn create_empty(&self) -> SystemTray {
        SystemTray::new()
    }

    pub fn handle(&self, app_handle: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a left click");
            }
            SystemTrayEvent::RightClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a right click");
            }
            SystemTrayEvent::DoubleClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a double click");
                let window = app_handle.get_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    tauri::async_runtime::block_on(async {
                        let app_state: State<'_, AppState> = app_handle.state();
                        let _ = app_state.disconnect().await;
                    });
                    app_handle.exit(0);
                }
                "show" => {
                    let window = app_handle.get_window("main");
                    if let Some(window) = window {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                _ => match id.split('-').collect::<Vec<_>>().as_slice() {
                    ["connect", server_name] => {
                        let app_handle = app_handle.clone();
                        let server_name = server_name.to_string();
                        tauri::async_runtime::spawn(async move {
                            let app_state: State<'_, AppState> = app_handle.state();
                            app_state
                                .connect_with_server_name(&server_name)
                                .await
                                .unwrap();
                        });
                    }
                    ["disconnect", _] => {
                        let app_handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let app_state: State<'_, AppState> = app_handle.state();
                            app_state.disconnect().await.unwrap();
                        });
                    }
                    _ => {}
                },
            },

            _ => {}
        }
    }
}
