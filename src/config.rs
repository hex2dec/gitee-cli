use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = "gitee";
const FALLBACK_CONFIG_DIR_NAME: &str = ".gitee";

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

        let config = self.load_config()?;
        Ok(config.and_then(|config| {
            config.token.map(|token| ResolvedToken {
                token,
                source: TokenSource::Config,
            })
        }))
    }

    pub fn save_token(&self, token: &str) -> Result<(), ConfigError> {
        let mut config = self.load_config()?.unwrap_or_default();
        config.token = Some(token.to_string());
        self.write_config(&config)
    }

    pub fn clear_token(&self) -> Result<(), ConfigError> {
        let path = self.config_path_buf();
        if !path.exists() {
            return Ok(());
        }

        let mut config = self.load_config()?.unwrap_or_default();
        config.token = None;

        if config.is_empty() {
            return fs::remove_file(path).map_err(ConfigError::Io);
        }

        self.write_config(&config)
    }

    pub fn load_clone_protocol(&self) -> Result<Option<CloneProtocol>, ConfigError> {
        let config = self.load_config()?;
        Ok(config.and_then(|config| config.clone_protocol))
    }

    pub fn save_clone_protocol(&self, protocol: CloneProtocol) -> Result<(), ConfigError> {
        let mut config = self.load_config()?.unwrap_or_default();
        config.clone_protocol = Some(protocol);
        self.write_config(&config)
    }

    pub fn config_path(&self) -> String {
        self.config_path_buf().display().to_string()
    }

    fn load_config(&self) -> Result<Option<ConfigFile>, ConfigError> {
        let path = self.config_path_buf();
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config = toml::from_str::<ConfigFile>(&contents).map_err(ConfigError::Toml)?;
        Ok(Some(config))
    }

    fn config_path_buf(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    fn write_config(&self, config: &ConfigFile) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.config_dir).map_err(ConfigError::Io)?;

        let contents = toml::to_string(config).map_err(ConfigError::TomlSerialize)?;
        fs::write(self.config_path_buf(), contents).map_err(ConfigError::Io)
    }
}

pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

pub enum TokenSource {
    Env,
    Config,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CloneProtocol {
    Https,
    Ssh,
}

impl TokenSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Config => "config",
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct ConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clone_protocol: Option<CloneProtocol>,
}

impl ConfigFile {
    fn is_empty(&self) -> bool {
        self.token.is_none() && self.clone_protocol.is_none()
    }
}

pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    TomlSerialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Toml(err) => write!(f, "{err}"),
            Self::TomlSerialize(err) => write!(f, "{err}"),
        }
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
