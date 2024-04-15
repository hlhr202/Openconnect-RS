use crate::cli::SeverConfigArgs;
use base64::Engine;
use comfy_table::Table;
use openconnect_core::storage::{OidcServer, PasswordServer, StoredConfigs, StoredServer};
use std::{error::Error, path::PathBuf};

pub async fn read_server_config_from_fs(
    server_name: &str,
    config_file: PathBuf,
) -> Result<(StoredServer, StoredConfigs), Box<dyn Error>> {
    let mut stored_configs = StoredConfigs::new(None, config_file);
    let config = stored_configs.read_from_file().await?;
    let server = config.servers.get(server_name);

    match server {
        Some(server) => {
            match server {
                StoredServer::Oidc(OidcServer { server, .. }) => {
                    println!("Connecting to OIDC server: {}", server_name);
                    println!("Server host: {}", server);
                }
                StoredServer::Password(PasswordServer { server, .. }) => {
                    println!("Connecting to password server: {}", server_name);
                    println!("Server host: {}", server);
                }
            }
            Ok((server.clone(), stored_configs))
        }
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Server {} not found", server_name),
        ))?,
    }
}

fn add_server_internal(stored_server: StoredServer) {
    let config_file = StoredConfigs::getorinit_config_file().expect("Failed to get config file");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    runtime.block_on(async {
        let mut stored_configs = StoredConfigs::new(None, config_file);

        stored_configs
            .read_from_file()
            .await
            .expect("Failed to read config file");

        stored_configs
            .upsert_server(stored_server)
            .await
            .expect("Failed to add server");
    });
}

pub fn request_add_server(server_config: SeverConfigArgs) {
    let new_server = match server_config {
        SeverConfigArgs::Oidc {
            name,
            server,
            issuer,
            client_id,
            client_secret,
            allow_insecure,
        } => {
            let oidc_server = OidcServer {
                name,
                server,
                issuer,
                client_id,
                client_secret,
                allow_insecure,
                updated_at: None,
            };

            StoredServer::Oidc(oidc_server)
        }
        SeverConfigArgs::Password {
            name,
            server,
            username,
            password,
            allow_insecure,
        } => {
            let password_server = PasswordServer {
                name,
                server,
                username,
                password: Some(password),
                allow_insecure,
                updated_at: None,
            };

            StoredServer::Password(password_server)
        }
    };

    add_server_internal(new_server);
}

pub fn request_delete_server(name: &str) {
    let config_file = StoredConfigs::getorinit_config_file().expect("Failed to get config file");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    runtime.block_on(async {
        let mut stored_configs = StoredConfigs::new(None, config_file);

        stored_configs
            .read_from_file()
            .await
            .expect("Failed to read config file");

        stored_configs
            .remove_server(name)
            .await
            .expect("Failed to delete server");
    });
}

pub fn request_list_servers() {
    let config_file = StoredConfigs::getorinit_config_file().expect("Failed to get config file");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    runtime.block_on(async {
        let mut stored_configs = StoredConfigs::new(None, config_file);

        let stored_configs = stored_configs.read_from_file().await.unwrap();
        let mut table = Table::new();
        table.set_header(vec![
            "Name".to_string(),
            "Type".to_string(),
            "Server".to_string(),
            "Allow Insecure".to_string(),
            "Updated At".to_string(),
        ]);

        for (name, server) in stored_configs.servers.iter() {
            match server {
                StoredServer::Oidc(OidcServer {
                    server,
                    allow_insecure,
                    updated_at,
                    ..
                }) => {
                    table.add_row(vec![
                        name.clone(),
                        "OIDC Server".to_string(),
                        server.clone(),
                        allow_insecure.unwrap_or(false).to_string(),
                        updated_at.as_ref().unwrap_or(&"".to_string()).to_owned(),
                    ]);
                }
                StoredServer::Password(PasswordServer {
                    server,
                    allow_insecure,
                    updated_at,
                    ..
                }) => {
                    table.add_row(vec![
                        name.clone(),
                        "Password Server".to_string(),
                        server.clone(),
                        allow_insecure.unwrap_or(false).to_string(),
                        updated_at.as_ref().unwrap_or(&"".to_string()).to_owned(),
                    ]);
                }
            }
        }

        println!("{table}");
    });
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "authType")]
pub enum SharableServer {
    #[serde(rename_all = "camelCase")]
    Oidc {
        server: String,
        allow_insecure: Option<bool>,
        issuer: String,
        client_id: String,
        client_secret: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Password {
        server: String,
        allow_insecure: Option<bool>,
    },
}

pub fn request_export_server(server_name: &str) {
    let config_file = StoredConfigs::getorinit_config_file().expect("Failed to get config file");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    runtime.block_on(async {
        let mut stored_configs = StoredConfigs::new(None, config_file);

        stored_configs
            .read_from_file()
            .await
            .expect("Failed to read config file");

        let server = stored_configs.servers.get(server_name);

        match server {
            Some(stored_server) => {
                let base64 = match stored_server {
                    StoredServer::Oidc(oidc_server) => {
                        let oidc_server = oidc_server.clone();
                        let partial_server = SharableServer::Oidc {
                            server: oidc_server.server,
                            allow_insecure: oidc_server.allow_insecure,
                            issuer: oidc_server.issuer,
                            client_id: oidc_server.client_id,
                            client_secret: oidc_server.client_secret,
                        };
                        let json =
                            serde_json::to_string(&partial_server).expect("Failed to serialize");
                        base64::prelude::BASE64_STANDARD.encode(json.as_bytes())
                    }
                    StoredServer::Password(password_server) => {
                        let password_server = password_server.clone();
                        let partial_server = SharableServer::Password {
                            server: password_server.server,
                            allow_insecure: password_server.allow_insecure,
                        };
                        let json =
                            serde_json::to_string(&partial_server).expect("Failed to serialize");
                        base64::prelude::BASE64_STANDARD.encode(json.as_bytes())
                    }
                };

                println!("Share this: {}", base64);
            }
            None => {
                eprintln!("Server {} not found", server_name);
            }
        }
    });
}

pub fn request_import_server(base64: &str) {
    let decoded = base64::prelude::BASE64_STANDARD
        .decode(base64.as_bytes())
        .expect("Failed to decode base64");

    let string = String::from_utf8(decoded).expect("Failed to convert to string");

    let server: SharableServer =
        serde_json::from_str(&string).expect("Failed to parse your import string");

    println!("==============================================");
    println!("Existing configs: {:#?}\n", server);

    let new_server = match server {
        SharableServer::Password {
            server,
            allow_insecure,
        } => {
            println!("We still need some extra information to complete the import");
            println!("==============================================\n");
            // prompt for servername, username and password

            println!("Enter an unique server name, this will be used as an identifier for the local config file");
            let name = dialoguer::Input::<String>::new()
                .with_prompt("Server name")
                .interact()
                .expect("Failed to get server name");

            let username = dialoguer::Input::<String>::new()
                .with_prompt("Enter username")
                .interact()
                .expect("Failed to get username");

            let password = dialoguer::Password::new()
                .with_prompt("Enter password")
                .interact()
                .expect("Failed to get password");

            StoredServer::Password(PasswordServer {
                name,
                server,
                username,
                password: Some(password),
                allow_insecure,
                updated_at: None,
            })
        }
        SharableServer::Oidc {
            server,
            allow_insecure,
            issuer,
            client_id,
            client_secret,
        } => {
            println!("We still need some information to complete the import");
            println!("==============================================\n");

            println!("Enter an unique server name, this will be used as an identifier for the local config file");
            let name = dialoguer::Input::<String>::new()
                .with_prompt("Server name")
                .interact()
                .expect("Failed to get server name");

            StoredServer::Oidc(OidcServer {
                name,
                server,
                issuer,
                client_id,
                client_secret,
                allow_insecure,
                updated_at: None,
            })
        }
    };

    add_server_internal(new_server);
}

#[test]
fn test_import_server() {
    let partial_import_server = SharableServer::Oidc {
        server: "https://example.com".to_string(),
        allow_insecure: Some(true),
        issuer: "https://example.com".to_string(),
        client_id: "12345".to_string(),
        client_secret: Some("123456".to_string()),
    };

    let json = serde_json::to_string(&partial_import_server).expect("Failed to serialize");
    let base64 = base64::prelude::BASE64_STANDARD.encode(json.as_bytes());
    request_import_server(&base64);
}
