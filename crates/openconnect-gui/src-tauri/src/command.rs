use crate::state::{AppState, StateError};
use openconnect_core::storage::{StoredConfigError, StoredConfigsJson, StoredServer};
use std::fmt::Display;

#[derive(serde::Serialize, thiserror::Error, Debug)]
pub struct ErrorResponse {
    code: String,
    message: String,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<StateError> for ErrorResponse {
    fn from(e: StateError) -> Self {
        let code = match e {
            StateError::ConfigError(_) => "CONFIG_ERROR",
            StateError::OpenconnectError(_) => "OPENCONNECT_ERROR",
            StateError::ChannelError(_) => "CHANNEL_ERROR",
            StateError::TauriError(_) => "TAURI_ERROR",
            StateError::OpenIdError(_) => "OPENID_ERROR",
            StateError::IoError(_) => "IO_ERROR",
        };
        Self {
            code: code.to_string(),
            message: e.to_string(),
        }
    }
}

impl From<StoredConfigError> for ErrorResponse {
    fn from(e: StoredConfigError) -> Self {
        let code = match e {
            StoredConfigError::BadInput(_) => "BAD_INPUT",
            StoredConfigError::ParseError(_) => "PARSE_ERROR",
            StoredConfigError::IoError(_) => "IO_ERROR",
        };
        Self {
            code: code.to_string(),
            message: e.to_string(),
        }
    }
}

#[tauri::command]
pub async fn connect_with_password(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> Result<(), ErrorResponse> {
    Ok(app_state.connect_with_user_pass(&server_name).await?)
}

#[tauri::command]
pub async fn connect_with_oidc(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> Result<(), ErrorResponse> {
    Ok(app_state.connect_with_oidc(&server_name).await?)
}

#[tauri::command]
pub async fn disconnect(app_state: tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
    Ok(app_state.disconnect().await?)
}

#[tauri::command]
pub async fn trigger_state_retrieve(
    app_state: tauri::State<'_, AppState>,
) -> Result<(), ErrorResponse> {
    Ok(app_state.trigger_state_retrieve().await?)
}

#[tauri::command]
pub async fn get_stored_configs(
    app_state: tauri::State<'_, AppState>,
) -> Result<StoredConfigsJson, ErrorResponse> {
    Ok(app_state.stored_configs.read().await.clone().into())
}

#[tauri::command]
pub async fn upsert_stored_server(
    app_state: tauri::State<'_, AppState>,
    server: StoredServer,
) -> Result<(), ErrorResponse> {
    let mut stored_configs = app_state.stored_configs.write().await;
    stored_configs.upsert_server(server).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_default_server(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> Result<(), ErrorResponse> {
    let mut stored_configs = app_state.stored_configs.write().await;
    stored_configs.set_default_server(&server_name).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_server(
    app_state: tauri::State<'_, AppState>,
    server_name: String,
) -> Result<(), ErrorResponse> {
    let mut stored_configs = app_state.stored_configs.write().await;
    stored_configs.remove_server(&server_name).await?;
    Ok(())
}
