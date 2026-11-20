use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
};

const XNONCE_LEN: usize = 24;
const AEAD_TAG_LEN: usize = 16;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredConfigsJson {
    default: Option<String>,
    servers: Vec<StoredServer>,
}

impl StoredConfigsJson {
    pub fn decrypted_by(&self, encryptor: &PassEncryptor) -> Self {
        let servers = self
            .servers
            .iter()
            .map(|server| match server {
                StoredServer::Oidc(oidc_server) => StoredServer::Oidc(oidc_server.clone()),
                StoredServer::Password(password_server) => {
                    StoredServer::Password(password_server.decrypted_by(encryptor))
                }
            })
            .collect();
        Self {
            default: self.default.clone(),
            servers,
        }
    }
}

fn parse_servers_map(
    json: StoredConfigsJson,
) -> Result<(Option<String>, HashMap<String, StoredServer>), StoredConfigError> {
    let mut servers = HashMap::new();
    for server in json.servers {
        let name = match &server {
            StoredServer::Oidc(OidcServer { name, .. }) => name,
            StoredServer::Password(PasswordServer { name, .. }) => name,
        };

        if servers.contains_key(name) {
            return Err(StoredConfigError::ParseError(format!(
                "Duplicated server name: {}, check your config file",
                name
            )));
        }

        servers.insert(name.clone(), server);
    }
    Ok((json.default, servers))
}

impl TryFrom<(StoredConfigsJson, PathBuf)> for StoredConfigs {
    type Error = StoredConfigError;

    fn try_from(
        (json, config_file): (StoredConfigsJson, PathBuf),
    ) -> Result<StoredConfigs, StoredConfigError> {
        let (default, servers) = parse_servers_map(json)?;
        let cipher = PassEncryptor::for_config_file(&config_file);
        Ok(StoredConfigs {
            default,
            servers,
            cipher,
            config_file,
            has_custom_key: false,
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
    pub allow_insecure: Option<bool>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordServer {
    pub name: String,
    pub server: String,
    pub username: String,
    pub password: Option<String>,
    pub allow_insecure: Option<bool>,
    pub updated_at: Option<String>,
}

impl PasswordServer {
    pub fn decrypted_by(&self, encryptor: &PassEncryptor) -> Self {
        let password = self
            .password
            .as_ref()
            .and_then(|p| encryptor.decrypt(p).ok());
        Self {
            name: self.name.clone(),
            server: self.server.clone(),
            username: self.username.clone(),
            password,
            allow_insecure: self.allow_insecure,
            updated_at: self.updated_at.clone(),
        }
    }

    pub fn encrypted_by(&self, encryptor: &PassEncryptor) -> Self {
        let password = self
            .password
            .as_ref()
            .and_then(|p| encryptor.encrypt(p).ok());
        Self {
            name: self.name.clone(),
            server: self.server.clone(),
            username: self.username.clone(),
            password,
            allow_insecure: self.allow_insecure,
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "authType")]
pub enum StoredServer {
    #[serde(rename_all = "camelCase")]
    Oidc(OidcServer),

    #[serde(rename_all = "camelCase")]
    Password(PasswordServer),
}

impl TryFrom<&StoredServer> for OidcServer {
    type Error = StoredConfigError;

    fn try_from(server: &StoredServer) -> Result<OidcServer, StoredConfigError> {
        match server {
            StoredServer::Oidc(oidc_server) => Ok(oidc_server.clone()),
            StoredServer::Password(_) => Err(StoredConfigError::ParseError(
                "Server is not OIDC type".to_string(),
            )),
        }
    }
}

impl TryFrom<&StoredServer> for PasswordServer {
    type Error = StoredConfigError;

    fn try_from(server: &StoredServer) -> Result<PasswordServer, StoredConfigError> {
        match server {
            StoredServer::Password(password_server) => Ok(password_server.clone()),
            StoredServer::Oidc(_) => Err(StoredConfigError::ParseError(
                "Server is not Password type".to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StoredConfigs {
    pub default: Option<String>,
    pub servers: HashMap<String, StoredServer>,
    pub cipher: PassEncryptor,
    pub config_file: PathBuf,
    has_custom_key: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum StoredConfigError {
    #[error("Bad input: {0}")]
    BadInput(String),

    #[error("Failed to parse stored config: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Cipher error")]
    CipherError(#[from] PassEncryptorError),
}

#[cfg(unix)]
fn set_unix_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Refusing to modify permissions on symlinked config path",
        ));
    }

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;

    // When invoked via `sudo` with preserved `HOME` (e.g., `openconnect-cli start`),
    // ensure newly created `~/.oidcvpn` / `config.json` / `.key` remain owned by the invoking user.
    // SAFETY: `geteuid` is always safe per POSIX.
    let euid = unsafe { libc::geteuid() };
    if euid == 0
        && let (Ok(uid_s), Ok(gid_s)) = (std::env::var("SUDO_UID"), std::env::var("SUDO_GID"))
        && let (Ok(uid), Ok(gid)) = (uid_s.parse::<u32>(), gid_s.parse::<u32>())
        && let Ok(cpath) = std::ffi::CString::new(path.as_os_str().as_bytes())
    {
        // SAFETY: `cpath` is a valid NUL-terminated path; `lchown` never follows symlinks.
        let _ = unsafe { libc::lchown(cpath.as_ptr(), uid, gid) };
    }

    Ok(())
}

#[cfg(not(unix))]
fn set_unix_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

impl StoredConfigs {
    pub fn new(pass_key: Option<String>, config_file: PathBuf) -> Self {
        let has_custom_key = pass_key.is_some();
        let cipher = match pass_key {
            Some(key) => PassEncryptor::new(Some(key)),
            None => PassEncryptor::for_config_file(&config_file),
        };
        Self {
            default: None,
            servers: HashMap::new(),
            cipher,
            config_file,
            has_custom_key,
        }
    }

    pub fn default_server(&self) -> Option<&StoredServer> {
        self.default
            .as_ref()
            .and_then(|name| self.servers.get(name))
    }

    pub fn getorinit_config_file() -> Result<PathBuf, StoredConfigError> {
        let home_dir = home::home_dir().ok_or_else(|| {
            StoredConfigError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Home directory not found",
            ))
        })?;

        let config_folder = home_dir.join(".oidcvpn");
        if !config_folder.exists() {
            std::fs::create_dir_all(&config_folder)?;
        }
        let _ = set_unix_mode(&config_folder, 0o700);

        let config_file = config_folder.join("config.json");
        if !config_file.exists() {
            std::fs::write(&config_file, br#"{"default":null,"servers":[]}"#)?;
        }
        let _ = set_unix_mode(&config_file, 0o600);

        Ok(config_file)
    }

    pub async fn save_to_file(&self) -> Result<&Self, StoredConfigError> {
        if let Some(parent) = self.config_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
            let _ = set_unix_mode(parent, 0o700);
        }

        let json = serde_json::to_string(&StoredConfigsJson::from(self.clone())).map_err(|e| {
            StoredConfigError::ParseError(format!("Failed to serialize config: {}", e))
        })?;

        tokio::fs::write(&self.config_file, json).await?;
        let _ = set_unix_mode(&self.config_file, 0o600);

        Ok(self)
    }

    pub async fn read_from_file(&mut self) -> Result<&mut Self, StoredConfigError> {
        let content = tokio::fs::read(&self.config_file).await?;
        let config_json: StoredConfigsJson = serde_json::from_slice(&content).map_err(|e| {
            StoredConfigError::ParseError(format!("Failed to parse config file: {}", e))
        })?;
        let (default, servers) = parse_servers_map(config_json)?;

        if !self.has_custom_key {
            self.cipher = PassEncryptor::for_config_file(&self.config_file);
        }
        self.default = default;
        self.servers = servers;

        Ok(self)
    }

    pub fn get_server_as_oidc_server(&self, name: &str) -> Result<&OidcServer, StoredConfigError> {
        self.servers
            .get(name)
            .and_then(|server| match server {
                StoredServer::Oidc(oidc) => Some(oidc),
                _ => None,
            })
            .ok_or_else(|| StoredConfigError::ParseError(format!("Server '{}' not found", name)))
    }

    pub fn get_server_as_password_server(
        &self,
        name: &str,
    ) -> Result<&PasswordServer, StoredConfigError> {
        self.servers
            .get(name)
            .and_then(|server| match server {
                StoredServer::Password(password_server) => Some(password_server),
                _ => None,
            })
            .ok_or_else(|| StoredConfigError::ParseError(format!("Server '{}' not found", name)))
    }

    pub async fn add_server(
        &mut self,
        server: StoredServer,
    ) -> Result<&mut Self, StoredConfigError> {
        let update_at = chrono::Utc::now().to_rfc3339();
        let mut server = server;
        let name = match &mut server {
            StoredServer::Oidc(oidc_server) => {
                oidc_server.updated_at = Some(update_at);
                oidc_server.name.to_owned()
            }
            StoredServer::Password(password_server) => {
                password_server.updated_at = Some(update_at);
                *password_server = password_server.encrypted_by(&self.cipher);
                password_server.name.to_owned()
            }
        };

        if let Entry::Vacant(e) = self.servers.entry(name.clone()) {
            e.insert(server);
            self.save_to_file().await?;
            Ok(self)
        } else {
            Err(StoredConfigError::BadInput(format!(
                "Server {} already exists",
                name
            )))
        }
    }

    pub async fn upsert_server(
        &mut self,
        server: StoredServer,
    ) -> Result<&mut Self, StoredConfigError> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        let mut server = server;
        let name = match &mut server {
            StoredServer::Oidc(oidc_server) => {
                oidc_server.updated_at = Some(updated_at);
                oidc_server.name.to_owned()
            }
            StoredServer::Password(password_server) => {
                password_server.updated_at = Some(updated_at);
                *password_server = password_server.encrypted_by(&self.cipher);
                password_server.name.to_owned()
            }
        };

        self.servers.insert(name, server);
        self.save_to_file().await?;
        Ok(self)
    }

    pub async fn remove_server(&mut self, name: &str) -> Result<&mut Self, StoredConfigError> {
        if self.default.as_ref().is_some_and(|d| d == name) {
            return Err(StoredConfigError::BadInput(format!(
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
            return Err(StoredConfigError::ParseError(format!(
                "Server {} not found",
                name
            )));
        }

        self.default = Some(name.to_string());
        self.save_to_file().await?;
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct PassEncryptor {
    secret: chacha20poly1305::Key,
    legacy_secret: Option<chacha20poly1305::Key>,
}

#[derive(Debug, thiserror::Error)]
pub enum PassEncryptorError {
    #[error("Cipher error: {0}")]
    CipherError(String),
}

impl Default for PassEncryptor {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PassEncryptor {
    fn derive_legacy_key(unique_key: &str) -> chacha20poly1305::Key {
        let mut hasher: sha2::Sha256 = sha2::digest::Digest::new();
        hasher.update(unique_key.as_bytes());
        let hash = hasher.finalize();
        let mut seed = rand::rngs::StdRng::from_seed(hash.into());
        XChaCha20Poly1305::generate_key(&mut seed)
    }

    fn load_or_create_random_key(key_file: &Path) -> std::io::Result<chacha20poly1305::Key> {
        if let Ok(hex_bytes) = std::fs::read_to_string(key_file)
            && let Ok(raw) = hex::decode(hex_bytes.trim())
            && raw.len() == 32
        {
            return Ok(*chacha20poly1305::Key::from_slice(&raw));
        }

        if let Some(parent) = key_file.parent() {
            std::fs::create_dir_all(parent)?;
            let _ = set_unix_mode(parent, 0o700);
        }

        let key = XChaCha20Poly1305::generate_key(&mut OsRng);
        std::fs::write(key_file, hex::encode(key))?;
        let _ = set_unix_mode(key_file, 0o600);
        Ok(key)
    }

    /// Create an encryptor backed by a per-installation random 32-byte key stored alongside
    /// `config_file` (mode `0600` on Unix), while retaining transparent decryption fallback
    /// for legacy v0.1 machine-UID encrypted entries.
    pub fn for_config_file(config_file: &Path) -> Self {
        let legacy_uid = machine_uid::get()
            .ok()
            .unwrap_or_else(|| "openconnect-rs-2024".to_string());
        let legacy_secret = Self::derive_legacy_key(&legacy_uid);

        let key_file = config_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".key");

        match Self::load_or_create_random_key(&key_file) {
            Ok(secret) => Self {
                secret,
                legacy_secret: Some(legacy_secret),
            },
            Err(_) => Self {
                secret: legacy_secret,
                legacy_secret: None,
            },
        }
    }

    pub fn new(unique_key: Option<String>) -> Self {
        if let Some(key) = unique_key {
            return Self {
                secret: Self::derive_legacy_key(&key),
                legacy_secret: None,
            };
        }

        let secret = XChaCha20Poly1305::generate_key(&mut OsRng);
        let legacy_uid = machine_uid::get()
            .ok()
            .unwrap_or_else(|| "openconnect-rs-2024".to_string());
        Self {
            secret,
            legacy_secret: Some(Self::derive_legacy_key(&legacy_uid)),
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, PassEncryptorError> {
        let cipher = XChaCha20Poly1305::new(&self.secret);
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).map_err(|e| {
            PassEncryptorError::CipherError(format!("Failed to encrypt password: {}", e))
        })?;
        let mut combined = Vec::with_capacity(XNONCE_LEN + encrypted.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&encrypted);
        Ok(hex::encode(combined))
    }

    fn decrypt_with_key(
        key: &chacha20poly1305::Key,
        raw: &[u8],
    ) -> Result<String, PassEncryptorError> {
        if raw.len() < XNONCE_LEN + AEAD_TAG_LEN {
            return Err(PassEncryptorError::CipherError(
                "Ciphertext is truncated or malformed".to_string(),
            ));
        }
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(&raw[..XNONCE_LEN]);
        let plaintext = cipher.decrypt(nonce, &raw[XNONCE_LEN..]).map_err(|e| {
            PassEncryptorError::CipherError(format!("Failed to decrypt password: {}", e))
        })?;
        String::from_utf8(plaintext).map_err(|e| {
            PassEncryptorError::CipherError(format!("Failed to decode decrypted password: {}", e))
        })
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String, PassEncryptorError> {
        let raw = hex::decode(ciphertext).map_err(|e| {
            PassEncryptorError::CipherError(format!("Failed to decode ciphertext hex: {}", e))
        })?;

        match Self::decrypt_with_key(&self.secret, &raw) {
            Ok(plaintext) => Ok(plaintext),
            Err(primary_err) => {
                if let Some(legacy_key) = &self.legacy_secret {
                    Self::decrypt_with_key(legacy_key, &raw).or(Err(primary_err))
                } else {
                    Err(primary_err)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_enc_roundtrip_and_truncated_input_never_panics() {
        let encryptor = PassEncryptor::new(Some("unit-test-key".to_string()));
        let password = "super-secret-password";
        let encrypted = encryptor.encrypt(password).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(password, decrypted);

        // Truncated hex inputs must return Err instead of panicking on `&raw[..24]`.
        assert!(encryptor.decrypt("").is_err());
        assert!(encryptor.decrypt("deadbeef").is_err());
        assert!(encryptor.decrypt(&"aa".repeat(24)).is_err());
    }

    #[tokio::test]
    async fn test_save_and_read_config_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config.json");

        let server = StoredServer::Oidc(OidcServer {
            name: "test".to_string(),
            server: "https://example.com".to_string(),
            issuer: "https://example.com".to_string(),
            client_id: "client_id".to_string(),
            client_secret: Some("client_secret".to_string()),
            allow_insecure: Some(true),
            updated_at: None,
        });

        let mut stored_config = StoredConfigs::new(None, config_file.clone());
        stored_config.upsert_server(server).await.unwrap();

        let mut reloaded = StoredConfigs::new(None, config_file.clone());
        reloaded.read_from_file().await.unwrap();
        assert!(reloaded.get_server_as_oidc_server("test").is_ok());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&config_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn test_config_json_serialization() {
        let server = StoredServer::Oidc(OidcServer {
            name: "oidc_server".to_string(),
            server: "https://example.com".to_string(),
            issuer: "https://example.com".to_string(),
            client_id: "client_id".to_string(),
            client_secret: None,
            allow_insecure: Some(true),
            updated_at: None,
        });

        let json = serde_json::to_string(&server).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["authType"], "oidc");
        assert_eq!(parsed["name"], "oidc_server");
        assert_eq!(parsed["allowInsecure"], true);

        let server = StoredServer::Password(PasswordServer {
            name: "password_server".to_string(),
            server: "https://example.com".to_string(),
            username: "username".to_string(),
            password: Some("password".to_string()),
            allow_insecure: Some(true),
            updated_at: None,
        });

        let json = serde_json::to_string(&server).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["authType"], "password");
        assert_eq!(parsed["name"], "password_server");
        assert_eq!(parsed["username"], "username");
    }

    #[tokio::test]
    async fn test_custom_pass_key_does_not_create_dot_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config.json");

        let server = StoredServer::Password(PasswordServer {
            name: "corp".to_string(),
            server: "https://vpn.example.com".to_string(),
            username: "alice".to_string(),
            password: Some("top-secret".to_string()),
            allow_insecure: Some(false),
            updated_at: None,
        });

        let mut writer = StoredConfigs::new(Some("custom-passphrase".into()), config_file.clone());
        writer.upsert_server(server).await.unwrap();
        assert!(!dir.path().join(".key").exists());

        let mut reader = StoredConfigs::new(Some("custom-passphrase".into()), config_file);
        reader.read_from_file().await.unwrap();
        assert!(!dir.path().join(".key").exists());

        let pw_srv = reader
            .get_server_as_password_server("corp")
            .unwrap()
            .decrypted_by(&reader.cipher);
        assert_eq!(pw_srv.password.as_deref(), Some("top-secret"));
    }
}
