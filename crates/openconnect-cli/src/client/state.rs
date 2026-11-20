use crate::{
    JsonRequest, JsonResponse,
    client::cert_prompt::{UntrustedCert, ask_trust_cert},
    sock,
};
use colored::Colorize;
use comfy_table::Table;
use futures::TryStreamExt;
use openconnect_core::{
    Connectable, VpnClient,
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    log::Logger,
    result::OpenconnectError,
    storage::{OidcServer, PasswordServer, StoredConfigs, StoredServer},
};
use openconnect_oidc::{
    obtain_cookie_by_oidc_token,
    oidc_device::{OpenIDDeviceAuth, OpenIDDeviceAuthConfig, OpenIDDeviceAuthError},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

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

/// Obtain a cookie using username/password. If the server certificate cannot be verified,
/// the user is asked on the terminal; the returned fingerprint (if any) was newly trusted by
/// the user and should be persisted.
pub async fn obtain_cookie_from_password_server(
    password_server: &PasswordServer,
    stored_configs: &StoredConfigs,
) -> Result<(Option<String>, Option<String>), StateError> {
    let password_server = password_server.decrypted_by(&stored_configs.cipher);

    let vpncscript = get_vpnc_script()?;

    let config = ConfigBuilder::default()
        .vpncscript(&vpncscript)
        .loglevel(LogLevel::Info)
        .build()?;

    let mut entrypoint_builder = EntrypointBuilder::new();
    entrypoint_builder
        .name(&password_server.name)
        .server(&password_server.server)
        .username(&password_server.username)
        .password(&password_server.password.clone().unwrap_or("".to_string()))
        .accept_insecure_cert(password_server.allow_insecure.unwrap_or(false))
        .enable_udp(true);
    if let Some(server_cert) = password_server.server_cert.as_deref() {
        entrypoint_builder.server_cert(server_cert);
    }
    let entrypoint = entrypoint_builder.build()?;

    let newly_trusted = Arc::new(Mutex::new(None::<String>));
    let newly_trusted_cb = Arc::clone(&newly_trusted);
    let event_handler = EventHandlers::default().with_handle_peer_cert_verify(move |prompt| {
        let accepted = ask_trust_cert(&UntrustedCert::from(prompt));
        if accepted && let Ok(mut slot) = newly_trusted_cb.lock() {
            *slot = Some(prompt.fingerprint.clone());
        }
        accepted
    });

    let client = VpnClient::new(config, event_handler)?;
    let client_clone = client.clone();

    let cookie =
        tokio::task::spawn_blocking(move || client_clone.connect_for_cookie(entrypoint)).await??;
    let newly_trusted = newly_trusted.lock().ok().and_then(|mut slot| slot.take());
    Ok((cookie, newly_trusted))
}

/// Persist a user-approved server certificate fingerprint.
async fn save_trusted_cert(stored_configs: &mut StoredConfigs, name: &str, fingerprint: &str) {
    match stored_configs
        .set_server_cert(name, Some(fingerprint))
        .await
    {
        Ok(_) => println!("Saved trusted certificate fingerprint for server {name}"),
        Err(e) => eprintln!(
            "{}",
            format!("Failed to save trusted certificate fingerprint: {e}").red()
        ),
    }
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

    Ok(obtain_cookie_by_oidc_token(
        &oidc_server.server,
        &token,
        oidc_server.allow_insecure.unwrap_or(false),
    )
    .await)
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
                                vec!["Server Name".to_string(), server_name],
                                vec!["Server URL".to_string(), server_url],
                                vec!["Server IP".to_string(), hostname],
                                vec!["Connection Status".to_string(), status],
                            ];

                            if let Some(info) = info {
                                let addr = info.addr.unwrap_or_default();
                                let netmask = info.netmask.unwrap_or_default();
                                let addr6 = info.addr6.unwrap_or_default();
                                let netmask6 = info.netmask6.unwrap_or_default();
                                let dns1 = info.dns[0].clone().unwrap_or_default();
                                let dns2 = info.dns[1].clone().unwrap_or_default();
                                let dns3 = info.dns[2].clone().unwrap_or_default();
                                let nbns1 = info.nbns[0].clone().unwrap_or_default();
                                let nbns2 = info.nbns[1].clone().unwrap_or_default();
                                let nbns3 = info.nbns[2].clone().unwrap_or_default();
                                let domain = info.domain.unwrap_or_default();
                                let proxy_pac = info.proxy_pac.unwrap_or_default();
                                let mtu = info.mtu.to_string();
                                let gateway_addr = info.gateway_addr.unwrap_or_default();
                                let info_rows = vec![
                                    vec!["IPv4 Address".to_string(), addr],
                                    vec!["IPv4 Netmask".to_string(), netmask],
                                    vec!["IPv6 Address".to_string(), addr6],
                                    vec!["IPv6 Netmask".to_string(), netmask6],
                                    vec!["DNS 1".to_string(), dns1],
                                    vec!["DNS 2".to_string(), dns2],
                                    vec!["DNS 3".to_string(), dns3],
                                    vec!["NBNS 1".to_string(), nbns1],
                                    vec!["NBNS 2".to_string(), nbns2],
                                    vec!["NBNS 3".to_string(), nbns3],
                                    vec!["Domain".to_string(), domain],
                                    vec!["Proxy PAC".to_string(), proxy_pac],
                                    vec!["MTU".to_string(), mtu],
                                    vec!["Gateway Address".to_string(), gateway_addr],
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
            Ok((stored_server, mut stored_configs)) => {
                let mut server_cert = stored_server.server_cert().map(str::to_string);
                let (cookie, name, server, allow_insecure) = match stored_server {
                    StoredServer::Password(password_server) => {
                        let cookie = crate::client::state::obtain_cookie_from_password_server(
                            &password_server,
                            &stored_configs,
                        )
                        .await;

                        let cookie = match cookie {
                            Ok((cookie, newly_trusted)) => {
                                if let Some(fingerprint) = newly_trusted {
                                    save_trusted_cert(
                                        &mut stored_configs,
                                        &password_server.name,
                                        &fingerprint,
                                    )
                                    .await;
                                    server_cert = Some(fingerprint);
                                }
                                cookie
                            }
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

                    loop {
                        unix_client
                            .send(JsonRequest::Start {
                                name: name.clone(),
                                server: server.clone(),
                                allow_insecure: allow_insecure.unwrap_or(false),
                                cookie: cookie.clone(),
                                server_cert: server_cert.clone(),
                            })
                            .await
                            .expect("Failed to send start command");

                        let Ok(Some(response)) = unix_client.framed_reader.try_next().await else {
                            break;
                        };
                        match response {
                            JsonResponse::StartResult {
                                name,
                                success,
                                err_message,
                                untrusted_cert,
                            } => {
                                if success {
                                    println!("\nStarted connection to server: {}", name);
                                    break;
                                }

                                // The daemon could not verify the certificate (e.g. first OIDC
                                // connection): ask the user, persist and retry.
                                if let Some(cert) = untrusted_cert {
                                    let fingerprint = cert.fingerprint.clone();
                                    if tokio::task::spawn_blocking(move || ask_trust_cert(&cert))
                                        .await
                                        .unwrap_or(false)
                                    {
                                        save_trusted_cert(&mut stored_configs, &name, &fingerprint)
                                            .await;
                                        server_cert = Some(fingerprint);
                                        continue;
                                    }
                                    let _ = unix_client.send(JsonRequest::Stop).await;
                                }

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
                            _ => {
                                eprintln!("{}", "\nReceived unexpected response".red());
                                break;
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
