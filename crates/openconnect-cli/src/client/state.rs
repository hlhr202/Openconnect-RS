use crate::{sock, JsonRequest, JsonResponse};
use comfy_table::Table;
use futures::TryStreamExt;
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    storage::{PasswordServer, StoredConfigs},
    Connectable, VpnClient,
};

pub fn get_vpnc_script() -> anyhow::Result<String> {
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
) -> anyhow::Result<Option<String>> {
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

    let result =
        tokio::task::spawn_blocking(move || client_clone.connect_for_cookie(entrypoint)).await;

    match result {
        Ok(Ok(cookie)) => Ok(cookie),
        Ok(Err(e)) => Err(e.into()),
        Err(e) => Err(e.into()),
    }
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
                eprintln!("Failed to connect to server: {}", e);
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
                        JsonResponse::StopResult { server_name } => {
                            println!("Stopped connection to server: {}", server_name)
                        }
                        _ => {
                            println!("Received unexpected response");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to server: {}", e);
                std::process::exit(1);
            }
        };
    });
}
