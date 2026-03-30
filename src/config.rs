use std::env;
use std::path::PathBuf;

use crate::credentials::{
    CredentialError, CredentialStore, ResolvedToken, TokenSource, credential_store_for_process,
};

const CONFIG_DIR_NAME: &str = "gitee";
const FALLBACK_CONFIG_DIR_NAME: &str = ".gitee";

pub struct ConfigStore {
    config_dir: PathBuf,
    credentials: Box<dyn CredentialStore>,
}

impl ConfigStore {
    pub fn from_env() -> Self {
        let config_dir = config_dir();
        Self {
            credentials: credential_store_for_process(&config_dir),
            config_dir,
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

        Ok(self
            .credentials
            .load_token()
            .map_err(ConfigError::store)?
            .map(|token| ResolvedToken {
                token,
                source: self.credentials.token_source(),
            }))
    }

    pub fn save_token(&self, token: &str) -> Result<TokenSource, ConfigError> {
        self.credentials
            .save_token(token)
            .map_err(ConfigError::store)?;
        Ok(self.credentials.token_source())
    }

    pub fn clear_token(&self) -> Result<(), ConfigError> {
        self.credentials.clear_token().map_err(ConfigError::store)
    }

    pub fn config_path(&self) -> String {
        self.config_path_buf().display().to_string()
    }
    fn config_path_buf(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
}

pub enum ConfigError {
    Store(CredentialError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(err) => write!(f, "{err}"),
        }
    }
}

impl ConfigError {
    fn store(error: CredentialError) -> Self {
        Self::Store(error)
    }
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
