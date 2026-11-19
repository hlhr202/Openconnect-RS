use crate::state::AppState;
use openconnect_core::storage::{OidcServer, PasswordServer, StoredServer};
use std::sync::Arc;
use tauri::{
    AppHandle, Manager, State,
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub const MAIN_TRAY_ID: &str = "main-tray";

pub struct AppSystemTray;

impl AppSystemTray {
    pub fn new() -> Arc<AppSystemTray> {
        Arc::new(AppSystemTray)
    }

    pub fn init_tray(&self, app_handle: &AppHandle) -> Result<(), tauri::Error> {
        let icon = Image::from_bytes(include_bytes!("../icons/disconnected-w.png"))?;
        let menu = MenuBuilder::new(app_handle).build()?;

        TrayIconBuilder::with_id(MAIN_TRAY_ID)
            .icon(icon)
            .menu(&menu)
            .on_menu_event(|app, event| {
                Self::handle_menu_event(app, event.id().as_ref());
            })
            .on_tray_icon_event(|tray, event| {
                Self::handle_tray_event(tray.app_handle(), event);
            })
            .build(app_handle)?;

        Ok(())
    }

    pub async fn recreate(&self, app_handle: &AppHandle) -> Result<(), tauri::Error> {
        let app_state: State<'_, AppState> = app_handle.state();
        let stored_configs = app_state.stored_configs.read().await;
        let servers = stored_configs.servers.values().collect::<Vec<_>>();
        let (status, current_server_name) =
            app_state.get_status_and_name().await.unwrap_or_else(|_| {
                (
                    crate::state::StatusPayload {
                        status: "INITIALIZED".into(),
                        message: None,
                    },
                    None,
                )
            });

        let Some(tray) = app_handle.tray_by_id(MAIN_TRAY_ID) else {
            return Ok(());
        };

        let icon_bytes: &[u8] = if status.status == "CONNECTED" {
            include_bytes!("../icons/connected-w.png")
        } else {
            include_bytes!("../icons/disconnected-w.png")
        };
        let icon = Image::from_bytes(icon_bytes)?;
        tray.set_icon(Some(icon))?;

        let mut submenu_builder = SubmenuBuilder::new(app_handle, "Servers");
        for server in &servers {
            let server_name = match server {
                StoredServer::Oidc(OidcServer { name, .. }) => name.as_str(),
                StoredServer::Password(PasswordServer { name, .. }) => name.as_str(),
            };
            let is_server_connected =
                current_server_name.as_deref() == Some(server_name) && status.status == "CONNECTED";

            let item = if is_server_connected {
                MenuItemBuilder::with_id(
                    format!("disconnect-{}", server_name),
                    format!("Disconnect {}", server_name),
                )
                .build(app_handle)?
            } else {
                MenuItemBuilder::with_id(
                    format!("connect-{}", server_name),
                    format!("Connect {}", server_name),
                )
                .enabled(status.status != "CONNECTED")
                .build(app_handle)?
            };
            submenu_builder = submenu_builder.item(&item);
        }

        let servers_submenu = submenu_builder.build()?;
        let show = MenuItemBuilder::with_id("show", "Show Window").build(app_handle)?;
        let quit = MenuItemBuilder::with_id("quit", "Quit").build(app_handle)?;

        let tray_menu = MenuBuilder::new(app_handle)
            .item(&servers_submenu)
            .separator()
            .item(&show)
            .item(&quit)
            .build()?;

        tray.set_menu(Some(tray_menu))?;
        Ok(())
    }

    fn handle_tray_event(app_handle: &AppHandle, event: TrayIconEvent) {
        if let TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
        | TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
            && let Some(window) = app_handle.get_webview_window("main")
        {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn handle_menu_event(app_handle: &AppHandle, id: &str) {
        match id {
            "quit" => {
                tauri::async_runtime::block_on(async {
                    let app_state: State<'_, AppState> = app_handle.state();
                    let _ = app_state.disconnect().await;
                });
                app_handle.exit(0);
            }
            "show" => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            other => {
                if let Some(server_name) = other.strip_prefix("connect-") {
                    let app_handle = app_handle.clone();
                    let server_name = server_name.to_string();
                    tauri::async_runtime::spawn(async move {
                        let app_state: State<'_, AppState> = app_handle.state();
                        if let Err(err) = app_state.connect_with_server_name(&server_name).await {
                            eprintln!("Failed to connect to {}: {}", server_name, err);
                        }
                    });
                } else if other.strip_prefix("disconnect-").is_some() {
                    let app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let app_state: State<'_, AppState> = app_handle.state();
                        if let Err(err) = app_state.disconnect().await {
                            eprintln!("Failed to disconnect: {}", err);
                        }
                    });
                }
            }
        }
    }
}
