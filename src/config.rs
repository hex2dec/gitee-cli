use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use keyring::{Entry, Error as KeyringError};

const CONFIG_DIR_NAME: &str = "gitee";
const FALLBACK_CONFIG_DIR_NAME: &str = ".gitee";
const KEYRING_SERVICE_NAME: &str = "gitee-cli:gitee.com";
const KEYRING_ACCOUNT_NAME: &str = "default";
const TEST_CREDENTIAL_STORE_DIR_ENV: &str = "GITEE_TEST_CREDENTIAL_STORE_DIR";

pub struct ConfigStore {
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn from_env() -> Self {
        Self {
            config_dir: config_dir(),
        }
    }

    pub fn load_runtime_token(&self) -> Result<Option<ResolvedToken>, ConfigError> {
        if let Ok(token) = env::var("GITEE_TOKEN") {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Ok(Some(ResolvedToken {
                    token,
                    source: TokenSource::Env,
                }));
            }
        }

        Ok(self.load_saved_token()?.map(|token| ResolvedToken {
            token,
            source: TokenSource::Keyring,
        }))
    }

    pub fn save_token(&self, token: &str) -> Result<TokenSource, ConfigError> {
        if let Some(path) = self.credential_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(ConfigError::Io)?;
            }
            fs::write(path, token).map_err(ConfigError::Io)?;
            return Ok(TokenSource::Keyring);
        }

        native_entry()?
            .set_password(token)
            .map_err(ConfigError::Keyring)?;
        Ok(TokenSource::Keyring)
    }

    pub fn clear_token(&self) -> Result<(), ConfigError> {
        if let Some(path) = self.credential_path() {
            return match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
                Err(err) => Err(ConfigError::Io(err)),
            };
        }

        match native_entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Ok(()),
            Err(err) => Err(ConfigError::Keyring(err)),
        }
    }

    pub fn config_path(&self) -> String {
        self.config_path_buf().display().to_string()
    }

    fn load_saved_token(&self) -> Result<Option<String>, ConfigError> {
        if let Some(path) = self.credential_path() {
            return match fs::read_to_string(path) {
                Ok(token) => Ok(Some(token.trim().to_string())),
                Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
                Err(err) => Err(ConfigError::Io(err)),
            };
        }

        match native_entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(err) => Err(ConfigError::Keyring(err)),
        }
    }

    fn config_path_buf(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    fn credential_path(&self) -> Option<PathBuf> {
        explicit_test_credential_store_dir()
            .or_else(synthetic_test_credential_store_dir)
            .map(|path| path.join("credentials.token"))
    }
}

fn explicit_test_credential_store_dir() -> Option<PathBuf> {
    env::var(TEST_CREDENTIAL_STORE_DIR_ENV)
        .ok()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn synthetic_test_credential_store_dir() -> Option<PathBuf> {
    if !running_under_test_harness() {
        return None;
    }

    if let Ok(path) = env::var("GITEE_CONFIG_DIR") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path).join(".gitee-test-store"));
        }
    }

    if let Ok(path) = env::var("GITEE_BASE_URL") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(
                env::temp_dir()
                    .join("gitee-cli-test-store")
                    .join(sanitize_path_component(path)),
            );
        }
    }

    if let Some(home_dir) = home_dir() {
        return Some(home_dir.join(".gitee-test-store"));
    }

    env::current_dir()
        .ok()
        .map(|path| path.join(".gitee-test-store"))
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

pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

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

pub enum ConfigError {
    Io(std::io::Error),
    Keyring(KeyringError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Keyring(err) => write!(f, "{err}"),
        }
    }
}

fn native_entry() -> Result<Entry, ConfigError> {
    Entry::new(KEYRING_SERVICE_NAME, KEYRING_ACCOUNT_NAME).map_err(ConfigError::Keyring)
}

fn config_dir() -> PathBuf {
    if let Ok(path) = env::var("GITEE_CONFIG_DIR") {
        let path = path.trim();
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }

    if let Ok(path) = env::var("XDG_CONFIG_HOME") {
        let path = path.trim();
        if !path.is_empty() {
            return PathBuf::from(path).join(CONFIG_DIR_NAME);
        }
    }

    if let Some(home_dir) = home_dir() {
        return home_dir.join(".config").join(CONFIG_DIR_NAME);
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(FALLBACK_CONFIG_DIR_NAME)
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
