use crate::{sock, JsonRequest, JsonResponse};
use colored::Colorize;
use comfy_table::Table;
use futures::TryStreamExt;
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    log::Logger,
    result::OpenconnectError,
    storage::{OidcServer, PasswordServer, StoredConfigs, StoredServer},
    Connectable, VpnClient,
};
use openconnect_oidc::{
    obtain_cookie_by_oidc_token,
    oidc_device::{OpenIDDeviceAuth, OpenIDDeviceAuthConfig, OpenIDDeviceAuthError},
};
use std::path::PathBuf;

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Openconnect error: {0}")]
    OpenconnectError(#[from] OpenconnectError),

    #[error("Tokio task error: {0}")]
    TokioTaskError(#[from] tokio::task::JoinError),

    #[error("OpenID device auth error: {0}")]
    OpenIDAuthError(#[from] OpenIDDeviceAuthError),
}

pub fn get_vpnc_script() -> Result<String, StateError> {
    let homedir = home::home_dir().ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Failed to get home directory",
    ))?;
    let vpncscript = homedir.join(".oidcvpn/bin/vpnc-script");
    let vpncscript = vpncscript.to_str().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Failed to get vpnc-script path as string",
    ))?;

    Ok(vpncscript.to_string())
}

pub async fn obtain_cookie_from_password_server(
    password_server: &PasswordServer,
    stored_configs: &StoredConfigs,
) -> Result<Option<String>, StateError> {
    let password_server = password_server.decrypted_by(&stored_configs.cipher);

    let vpncscript = get_vpnc_script()?;

    let config = ConfigBuilder::default()
        .vpncscript(&vpncscript)
        .loglevel(LogLevel::Info)
        .build()?;

    let entrypoint = EntrypointBuilder::new()
        .name(&password_server.name)
        .server(&password_server.server)
        .username(&password_server.username)
        .password(&password_server.password.clone().unwrap_or("".to_string()))
        .accept_insecure_cert(password_server.allow_insecure.unwrap_or(false))
        .enable_udp(true)
        .build()?;

    let event_handler = EventHandlers::default();

    let client = VpnClient::new(config, event_handler)?;
    let client_clone = client.clone();

    Ok(tokio::task::spawn_blocking(move || client_clone.connect_for_cookie(entrypoint)).await??)
}

pub async fn obtain_cookie_from_oidc_server(
    oidc_server: &OidcServer,
    _stored_configs: &StoredConfigs,
) -> Result<Option<String>, StateError> {
    let openid_config = OpenIDDeviceAuthConfig {
        issuer_url: oidc_server.issuer.clone(),
        client_id: oidc_server.client_id.clone(),
        client_secret: oidc_server.client_secret.clone(),
    };

    let mut openid = OpenIDDeviceAuth::new(openid_config).await?;
    let device_auth_response = openid.exchange_device_token().await?;
    let verification_url = device_auth_response.verification_uri().url();
    let user_code = device_auth_response.user_code();
    println!(
        "Please visit {} and enter code {}",
        verification_url,
        user_code.secret()
    );

    let token = openid
        .exchange_token(&device_auth_response, tokio::time::sleep, None)
        .await?;

    Ok(obtain_cookie_by_oidc_token(&oidc_server.server, &token).await)
}

pub fn request_get_status() {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    runtime.block_on(async {
        let client = sock::UnixDomainClient::connect().await;

        match client {
            Ok(mut client) => {
                client
                    .send(JsonRequest::Info)
                    .await
                    .expect("Failed to send info command");

                if let Ok(Some(response)) = client.framed_reader.try_next().await {
                    match response {
                        JsonResponse::InfoResult {
                            server_name,
                            server_url,
                            hostname,
                            status,
                            info,
                        } => {
                            let mut table = Table::new();
                            let mut rows = vec![
                                vec![format!("Server Name"), server_name],
                                vec![format!("Server URL"), server_url],
                                vec![format!("Server IP"), hostname],
                                vec![format!("Connection Status"), status],
                            ];

                            if let Some(info) = info {
                                let addr = info.addr.unwrap_or("".to_string());
                                let netmask = info.netmask.unwrap_or("".to_string());
                                let addr6 = info.addr6.unwrap_or("".to_string());
                                let netmask6 = info.netmask6.unwrap_or("".to_string());
                                let dns1 = info.dns[0].clone().unwrap_or("".to_string());
                                let dns2 = info.dns[1].clone().unwrap_or("".to_string());
                                let dns3 = info.dns[2].clone().unwrap_or("".to_string());
                                let nbns1 = info.nbns[0].clone().unwrap_or("".to_string());
                                let nbns2 = info.nbns[1].clone().unwrap_or("".to_string());
                                let nbns3 = info.nbns[2].clone().unwrap_or("".to_string());
                                let domain = info.domain.unwrap_or("".to_string());
                                let proxy_pac = info.proxy_pac.unwrap_or("".to_string());
                                let mtu = info.mtu.to_string();
                                let gateway_addr =
                                    info.gateway_addr.clone().unwrap_or("".to_string());
                                let info_rows = vec![
                                    vec![format!("IPv4 Address"), addr],
                                    vec![format!("IPv4 Netmask"), netmask],
                                    vec![format!("IPv6 Address"), addr6],
                                    vec![format!("IPv6 Netmask"), netmask6],
                                    vec![format!("DNS 1"), dns1],
                                    vec![format!("DNS 2"), dns2],
                                    vec![format!("DNS 3"), dns3],
                                    vec![format!("NBNS 1"), nbns1],
                                    vec![format!("NBNS 2"), nbns2],
                                    vec![format!("NBNS 3"), nbns3],
                                    vec![format!("Domain"), domain],
                                    vec![format!("Proxy PAC"), proxy_pac],
                                    vec![format!("MTU"), mtu],
                                    vec![format!("Gateway Address"), gateway_addr],
                                ];

                                rows.extend(info_rows);
                            }

                            table.add_rows(rows);

                            println!("{table}");
                        }
                        _ => {
                            println!("Received unexpected response");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", format!("\nFailed to connect to server: {}", e).red());
                std::process::exit(1);
            }
        }
    });
}

pub fn request_start_server(name: String, config_file: PathBuf) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    runtime.block_on(async {
        Logger::init().expect("Failed to initialize logger");

        match crate::client::config::read_server_config_from_fs(&name, config_file).await {
            Ok((stored_server, stored_configs)) => {
                let (cookie, name, server, allow_insecure) = match stored_server {
                    StoredServer::Password(password_server) => {
                        let cookie = crate::client::state::obtain_cookie_from_password_server(
                            &password_server,
                            &stored_configs,
                        )
                        .await;

                        let cookie = match cookie {
                            Ok(cookie) => cookie,
                            Err(e) => {
                                tracing::error!("Failed to obtain cookie: {}", e);
                                None
                            }
                        };

                        (
                            cookie,
                            password_server.name,
                            password_server.server,
                            password_server.allow_insecure,
                        )
                    }
                    StoredServer::Oidc(oidc_server) => {
                        let cookie_res = crate::client::state::obtain_cookie_from_oidc_server(
                            &oidc_server,
                            &stored_configs,
                        )
                        .await;

                        let cookie = match cookie_res {
                            Ok(cookie) => cookie,
                            Err(e) => {
                                tracing::error!("Failed to obtain cookie: {}", e);
                                None
                            }
                        };

                        (
                            cookie,
                            oidc_server.name,
                            oidc_server.server,
                            oidc_server.allow_insecure,
                        )

                        // TODO: optimize error message handling
                    }
                };

                let mut unix_client = sock::UnixDomainClient::connect()
                    .await
                    .expect("Failed to connect to daemon");

                if let Some(cookie) = cookie {
                    println!("Obtained cookie from server");

                    unix_client
                        .send(JsonRequest::Start {
                            name,
                            server,
                            allow_insecure: allow_insecure.unwrap_or(false),
                            cookie,
                        })
                        .await
                        .expect("Failed to send start command");

                    if let Ok(Some(response)) = unix_client.framed_reader.try_next().await {
                        match response {
                            JsonResponse::StartResult {
                                name,
                                success,
                                err_message,
                            } => {
                                if success {
                                    println!("\nStarted connection to server: {}", name);
                                } else {
                                    eprintln!(
                                        "{}",
                                        format!(
                                            "\nFailed to start connection: {}",
                                            err_message.unwrap_or("Unknown error".to_string())
                                        )
                                        .red()
                                    );
                                    std::process::exit(1);
                                }
                            }
                            _ => {
                                eprintln!("{}", "\nReceived unexpected response".red());
                            }
                        }
                    }
                } else {
                    unix_client
                        .send(JsonRequest::Stop)
                        .await
                        .expect("Failed to send stop command");

                    eprintln!(
                        "{}",
                        "\nFailed to obtain cookie, check logs for more information".red()
                    ); // TODO: improve error message
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("{}", format!("\nFailed to get server: {}", e).red());
                std::process::exit(1);
            }
        }
    });
}

pub fn request_stop_server() {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    runtime.block_on(async {
        let client = sock::UnixDomainClient::connect().await;

        match client {
            Ok(mut client) => {
                client
                    .send(JsonRequest::Stop)
                    .await
                    .expect("Failed to send stop command");

                if let Ok(Some(response)) = client.framed_reader.try_next().await {
                    match response {
                        JsonResponse::StopResult { name: server_name } => {
                            println!("\nStopped connection to server: {}", server_name)
                        }
                        _ => {
                            println!("Received unexpected response");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", format!("\nFailed to connect to server: {}", e).red());
                std::process::exit(1);
            }
        };
    });
}
