use serde::{de::Error, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredConfigsJson {
    default: Option<String>,
    servers: Vec<StoredServer>,
}

impl TryFrom<StoredConfigsJson> for StoredConfigs {
    type Error = StoredConfigError;

    fn try_from(json: StoredConfigsJson) -> Result<StoredConfigs, StoredConfigError> {
        let mut servers = HashMap::new();
        for server in json.servers {
            let name = match &server {
                StoredServer::Oidc(OidcServer { name, .. }) => name,
                StoredServer::Password(PasswordServer { name, .. }) => name,
            };

            if servers.contains_key(name) {
                return Err(StoredConfigError::ParseError(serde_json::Error::custom(
                    format!("Duplicated server name: {}, check your config file", name),
                )));
            }

            servers.insert(name.clone(), server);
        }

        Ok(StoredConfigs {
            default: json.default,
            servers,
        })
    }
}

impl From<StoredConfigs> for StoredConfigsJson {
    fn from(config: StoredConfigs) -> StoredConfigsJson {
        StoredConfigsJson {
            default: config.default,
            servers: config.servers.into_values().collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcServer {
    pub name: String,
    pub server: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordServer {
    pub name: String,
    pub server: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "authType")]
pub enum StoredServer {
    #[serde(rename_all = "camelCase")]
    Oidc(OidcServer),

    #[serde(rename_all = "camelCase")]
    Password(PasswordServer),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredConfigs {
    pub default: Option<String>,
    pub servers: HashMap<String, StoredServer>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoredConfigError {
    #[error("File system error when read/write stored config: {0}")]
    FileSystemError(String),

    #[error("Failed to parse stored config: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl Default for StoredConfigs {
    fn default() -> Self {
        Self::new()
    }
}

impl StoredConfigs {
    pub fn new() -> Self {
        Self {
            default: None,
            servers: HashMap::new(),
        }
    }

    pub fn default_server(&self) -> Option<&StoredServer> {
        self.default
            .as_ref()
            .and_then(|name| self.servers.get(name))
    }

    pub async fn getorinit_config_file(&self) -> Result<PathBuf, StoredConfigError> {
        let home_dir = home::home_dir().ok_or(StoredConfigError::FileSystemError(
            "Failed to get home directory".to_string(),
        ))?;

        let config_folder = home_dir.join(".oidcvpn");
        if !config_folder.exists() {
            tokio::fs::create_dir(&config_folder).await.map_err(|e| {
                StoredConfigError::FileSystemError(format!("Failed to create config folder: {}", e))
            })?
        }

        let config_file = config_folder.join("config.json");
        if !config_file.exists() {
            tokio::fs::write(&config_file, br#"{"default":null,"servers":[]}"#)
                .await
                .map_err(|e| {
                    StoredConfigError::FileSystemError(format!(
                        "Failed to initialize config file: {}",
                        e
                    ))
                })?;
        }

        Ok(config_file)
    }

    pub async fn save_to_file(&self) -> Result<&Self, StoredConfigError> {
        let config_file = self.getorinit_config_file().await?;
        let json = serde_json::to_string(&StoredConfigsJson::from(self.clone()))?;

        tokio::fs::write(&config_file, json).await.map_err(|e| {
            StoredConfigError::FileSystemError(format!("Failed to write config file: {}", e))
        })?;

        Ok(self)
    }

    pub async fn read_from_file(&mut self) -> Result<&mut Self, StoredConfigError> {
        let config_file = self.getorinit_config_file().await?;
        let content = tokio::fs::read(&config_file).await.map_err(|e| {
            StoredConfigError::FileSystemError(format!("Failed to read config file: {}", e))
        })?;
        let config_json: StoredConfigsJson = serde_json::from_slice(&content)?;
        let config = StoredConfigs::try_from(config_json)?;

        self.default = config.default;
        self.servers = config.servers;

        Ok(self)
    }

    pub fn get_server_as_oidc(&self, name: &str) -> Option<&OidcServer> {
        self.servers.get(name).and_then(|server| match server {
            StoredServer::Oidc(oidc) => Some(oidc),
            _ => None,
        })
    }

    pub fn get_server_as_password(&self, name: &str) -> Option<&PasswordServer> {
        self.servers.get(name).and_then(|server| match server {
            StoredServer::Password(password) => Some(password),
            _ => None,
        })
    }

    pub async fn upsert_server(
        &mut self,
        server: StoredServer,
    ) -> Result<&mut Self, StoredConfigError> {
        let name = match &server {
            StoredServer::Oidc(OidcServer { name, .. }) => name,
            StoredServer::Password(PasswordServer { name, .. }) => name,
        };

        *self.servers.entry(name.clone()).or_insert(server) = server.clone();
        self.save_to_file().await?;
        Ok(self)
    }

    pub async fn remove_server(&mut self, name: &str) -> Result<&mut Self, StoredConfigError> {
        if self.default.as_ref().is_some_and(|d| d == name) {
            return Err(StoredConfigError::FileSystemError(format!(
                "Cannot remove default server {}",
                name
            )));
        }
        self.servers.remove(name);
        self.save_to_file().await?;
        Ok(self)
    }

    pub async fn set_default_server(&mut self, name: &str) -> Result<&mut Self, StoredConfigError> {
        if !self.servers.contains_key(name) {
            return Err(StoredConfigError::ParseError(serde_json::Error::custom(
                format!("Server {} not found", name),
            )));
        }

        self.default = Some(name.to_string());
        self.save_to_file().await?;
        Ok(self)
    }
}

#[tokio::test]
async fn test_read_config() {
    let mut stored_configs = StoredConfigs::new();
    stored_configs.read_from_file().await.unwrap();
    println!("parsed struct: {:#?}", stored_configs);

    let stored_configs_json = StoredConfigsJson::from(stored_configs.clone());
    let json = serde_json::to_string(&stored_configs_json).unwrap();
    println!("json: {}", json);
}

#[tokio::test]
async fn test_save_config() {
    let server = StoredServer::Oidc(OidcServer {
        name: "test".to_string(),
        server: "https://example.com".to_string(),
        issuer: "https://example.com".to_string(),
        client_id: "client_id".to_string(),
        client_secret: Some("client_secret".to_string()),
    });

    let mut stored_config = StoredConfigs::new();
    let config = stored_config
        .read_from_file()
        .await
        .unwrap()
        .upsert_server(server)
        .await
        .unwrap()
        .save_to_file()
        .await
        .unwrap();

    println!("saved: {:?}", config);
    println!(
        "read: {:?}",
        StoredConfigs::new().read_from_file().await.unwrap()
    );
}

#[tokio::test]
async fn test_config_type() {
    let server = StoredServer::Oidc(OidcServer {
        name: "oidc_server".to_string(),
        server: "https://example.com".to_string(),
        issuer: "https://example.com".to_string(),
        client_id: "client_id".to_string(),
        client_secret: None,
    });

    let json = serde_json::to_string(&server).unwrap();
    assert_eq!(
        json,
        r#"{"authType":"oidc","server":"https://example.com","issuer":"https://example.com","clientId":"client_id","clientSecret":null}"#
    );

    let server = StoredServer::Password(PasswordServer {
        name: "password_server".to_string(),
        server: "https://example.com".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    });

    let json = serde_json::to_string(&server).unwrap();
    assert_eq!(
        json,
        r#"{"authType":"password","server":"https://example.com","username":"username","password":"password"}"#
    );
}
