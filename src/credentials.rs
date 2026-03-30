use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use keyring::{Entry, Error as KeyringError};

const KEYRING_SERVICE_NAME: &str = "gitee-cli:gitee.com";
const KEYRING_ACCOUNT_NAME: &str = "default";

#[derive(Clone, Copy)]
pub enum TokenSource {
    Env,
    Keyring,
}

impl TokenSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Keyring => "keyring",
        }
    }
}

pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

pub trait CredentialStore {
    fn load_token(&self) -> Result<Option<String>, CredentialError>;
    fn save_token(&self, token: &str) -> Result<(), CredentialError>;
    fn clear_token(&self) -> Result<(), CredentialError>;
    fn token_source(&self) -> TokenSource;
}

pub fn credential_store_for_process(config_dir: &Path) -> Box<dyn CredentialStore> {
    if let Some(path) = synthetic_test_credential_path(config_dir) {
        return Box::new(FileCredentialStore::new(path, TokenSource::Keyring));
    }

    Box::new(KeyringCredentialStore::new())
}

pub enum CredentialError {
    Io(std::io::Error),
    Keyring(KeyringError),
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Keyring(err) => write!(f, "{err}"),
        }
    }
}

struct KeyringCredentialStore {
    service_name: &'static str,
    account_name: &'static str,
}

impl KeyringCredentialStore {
    fn new() -> Self {
        Self {
            service_name: KEYRING_SERVICE_NAME,
            account_name: KEYRING_ACCOUNT_NAME,
        }
    }

    fn entry(&self) -> Result<Entry, CredentialError> {
        Entry::new(self.service_name, self.account_name).map_err(CredentialError::Keyring)
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn load_token(&self) -> Result<Option<String>, CredentialError> {
        match self.entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(err) => Err(CredentialError::Keyring(err)),
        }
    }

    fn save_token(&self, token: &str) -> Result<(), CredentialError> {
        self.entry()?
            .set_password(token)
            .map_err(CredentialError::Keyring)
    }

    fn clear_token(&self) -> Result<(), CredentialError> {
        match self.entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Ok(()),
            Err(err) => Err(CredentialError::Keyring(err)),
        }
    }

    fn token_source(&self) -> TokenSource {
        TokenSource::Keyring
    }
}

struct FileCredentialStore {
    path: PathBuf,
    source: TokenSource,
}

impl FileCredentialStore {
    fn new(path: PathBuf, source: TokenSource) -> Self {
        Self { path, source }
    }
}

impl CredentialStore for FileCredentialStore {
    fn load_token(&self) -> Result<Option<String>, CredentialError> {
        match fs::read_to_string(&self.path) {
            Ok(token) => {
                let token = token.trim().to_string();
                if token.is_empty() {
                    return Ok(None);
                }
                Ok(Some(token))
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(CredentialError::Io(err)),
        }
    }

    fn save_token(&self, token: &str) -> Result<(), CredentialError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(CredentialError::Io)?;
        }
        fs::write(&self.path, token).map_err(CredentialError::Io)
    }

    fn clear_token(&self) -> Result<(), CredentialError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(CredentialError::Io(err)),
        }
    }

    fn token_source(&self) -> TokenSource {
        self.source
    }
}

fn synthetic_test_credential_path(config_dir: &Path) -> Option<PathBuf> {
    if !running_under_test_harness() {
        return None;
    }

    if env::var_os("GITEE_CONFIG_DIR").is_some() {
        return Some(config_dir.join(".gitee-test-store/credentials.token"));
    }

    if let Some(home_dir) = home_dir() {
        return Some(home_dir.join(".gitee-test-store/credentials.token"));
    }

    if let Ok(current_dir) = env::current_dir() {
        return Some(current_dir.join(".gitee-test-store/credentials.token"));
    }

    Some(
        env::temp_dir()
            .join("gitee-cli-test-store")
            .join(sanitize_path_component(&config_dir.display().to_string()))
            .join("credentials.token"),
    )
}

fn running_under_test_harness() -> bool {
    env::var_os("CARGO_BIN_EXE_gitee").is_some() || env::var_os("CARGO_TARGET_TMPDIR").is_some()
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(path) = env::var("HOME") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    if let Ok(path) = env::var("USERPROFILE") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    match (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
        (Ok(drive), Ok(path)) if !drive.trim().is_empty() && !path.trim().is_empty() => {
            Some(PathBuf::from(format!("{}{}", drive.trim(), path.trim())))
        }
        _ => None,
    }
}
