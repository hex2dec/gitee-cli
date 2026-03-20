use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_AUTH: u8 = 3;
const EXIT_CONFIG: u8 = 4;
const EXIT_REMOTE: u8 = 5;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(outcome) => {
            if let Some(body) = outcome.stdout {
                println!("{body}");
            }
            ExitCode::from(outcome.code)
        }
        Err(error) => {
            if let Some(body) = error.stdout {
                println!("{body}");
            }
            if let Some(message) = error.stderr {
                eprintln!("{message}");
            }
            ExitCode::from(error.code)
        }
    }
}

fn run(args: Vec<String>) -> Result<CommandOutcome, CommandError> {
    let Some((command, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing command"));
    };

    match command.as_str() {
        "auth" => run_auth(rest),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_auth(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing auth subcommand"));
    };

    match subcommand.as_str() {
        "status" => {
            let output = parse_output_format(rest)?;
            auth_status(output)
        }
        "login" => {
            let login_args = parse_auth_login_args(rest)?;
            auth_login(login_args)
        }
        "logout" => {
            let output = parse_output_format(rest)?;
            auth_logout(output)
        }
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn auth_status(output: OutputFormat) -> Result<CommandOutcome, CommandError> {
    let Some(credentials) = load_runtime_token().map_err(CommandError::config)? else {
        return Ok(render_auth_state(
            output,
            EXIT_AUTH,
            AuthState {
                authenticated: false,
                source: "none",
                username: None,
                logged_out: false,
            },
        ));
    };

    let username = fetch_current_user(&credentials.token).map_err(map_auth_error)?;
    Ok(render_auth_state(
        output,
        EXIT_OK,
        AuthState {
            authenticated: true,
            source: credentials.source.as_str(),
            username: Some(username),
            logged_out: false,
        },
    ))
}

fn auth_login(args: AuthLoginArgs) -> Result<CommandOutcome, CommandError> {
    let token = match args.token_source {
        LoginTokenSource::Flag(token) => token,
        LoginTokenSource::Stdin => read_token_from_stdin().map_err(CommandError::usage)?,
    };

    let username = fetch_current_user(&token).map_err(map_auth_error)?;
    save_config_token(&token).map_err(CommandError::config)?;

    Ok(render_auth_state(
        args.output,
        EXIT_OK,
        AuthState {
            authenticated: true,
            source: "config",
            username: Some(username),
            logged_out: false,
        },
    ))
}

fn auth_logout(output: OutputFormat) -> Result<CommandOutcome, CommandError> {
    clear_config_token().map_err(CommandError::config)?;
    Ok(render_auth_state(
        output,
        EXIT_OK,
        AuthState {
            authenticated: false,
            source: "none",
            username: None,
            logged_out: true,
        },
    ))
}

fn render_auth_state(output: OutputFormat, code: u8, state: AuthState) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            code,
            json!({
                "authenticated": state.authenticated,
                "source": state.source,
                "username": state.username,
                "logged_out": state.logged_out,
                "config_path": config_path(),
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(code, render_auth_text(&state)),
    }
}

fn render_auth_text(state: &AuthState) -> String {
    if state.logged_out {
        return "Logged out".to_string();
    }

    if !state.authenticated {
        return "Not authenticated".to_string();
    }

    match &state.username {
        Some(username) => format!("Authenticated as {username} via {}", state.source),
        None => format!("Authenticated via {}", state.source),
    }
}

fn parse_output_format(args: &[String]) -> Result<OutputFormat, CommandError> {
    let mut output = OutputFormat::Text;
    for arg in args {
        match arg.as_str() {
            "--json" => output = OutputFormat::Json,
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }
    Ok(output)
}

fn parse_auth_login_args(args: &[String]) -> Result<AuthLoginArgs, CommandError> {
    let mut output = OutputFormat::Text;
    let mut token: Option<LoginTokenSource> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--token" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --token"));
                };
                if token.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --token or --with-token",
                    ));
                }
                token = Some(LoginTokenSource::Flag(value.clone()));
                index += 2;
            }
            "--with-token" => {
                if token.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --token or --with-token",
                    ));
                }
                token = Some(LoginTokenSource::Stdin);
                index += 1;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    let Some(token_source) = token else {
        return Err(CommandError::usage(
            "login requires --token or --with-token",
        ));
    };

    Ok(AuthLoginArgs {
        output,
        token_source,
    })
}

fn load_runtime_token() -> Result<Option<ResolvedToken>, ConfigError> {
    if let Ok(token) = env::var("GITEE_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(Some(ResolvedToken {
                token,
                source: TokenSource::Env,
            }));
        }
    }

    let config = load_config()?;
    Ok(config.map(|config| ResolvedToken {
        token: config.token,
        source: TokenSource::Config,
    }))
}

fn load_config() -> Result<Option<ConfigFile>, ConfigError> {
    let path = config_path_buf();
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path).map_err(ConfigError::Io)?;
    let config = toml::from_str::<ConfigFile>(&contents).map_err(ConfigError::Toml)?;
    Ok(Some(config))
}

fn save_config_token(token: &str) -> Result<(), ConfigError> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(ConfigError::Io)?;

    let contents = toml::to_string(&ConfigFile {
        token: token.to_string(),
    })
    .map_err(ConfigError::TomlSerialize)?;

    fs::write(config_path_buf(), contents).map_err(ConfigError::Io)
}

fn clear_config_token() -> Result<(), ConfigError> {
    let path = config_path_buf();
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).map_err(ConfigError::Io)
}

fn fetch_current_user(token: &str) -> Result<String, AuthError> {
    let response = Client::new()
        .get(format!("{}/v5/user", base_url()))
        .query(&[("access_token", token)])
        .send()
        .map_err(AuthError::Transport)?;

    if response.status().is_success() {
        let user = response
            .json::<UserResponse>()
            .map_err(AuthError::Transport)?;
        return Ok(user.login);
    }

    if matches!(response.status().as_u16(), 400 | 401) {
        return Err(AuthError::InvalidToken);
    }

    Err(AuthError::UnexpectedStatus(response.status().as_u16()))
}

fn read_token_from_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read token from stdin: {err}"))?;

    let token = input.trim().to_string();
    if token.is_empty() {
        return Err("token from stdin cannot be empty".to_string());
    }

    Ok(token)
}

fn base_url() -> String {
    env::var("GITEE_BASE_URL").unwrap_or_else(|_| "https://gitee.com".to_string())
}

fn config_path() -> String {
    config_path_buf().display().to_string()
}

fn config_path_buf() -> PathBuf {
    config_dir().join("config.toml")
}

fn config_dir() -> PathBuf {
    if let Ok(path) = env::var("GITEE_CONFIG_DIR") {
        return PathBuf::from(path);
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".gitee-cli")
}

fn map_auth_error(error: AuthError) -> CommandError {
    match error {
        AuthError::InvalidToken => CommandError {
            code: EXIT_AUTH,
            stdout: None,
            stderr: Some("authentication failed".to_string()),
        },
        AuthError::Transport(err) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed: {err}")),
        },
        AuthError::UnexpectedStatus(status) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!(
                "remote request returned unexpected status: {status}"
            )),
        },
    }
}

struct CommandOutcome {
    code: u8,
    stdout: Option<String>,
}

impl CommandOutcome {
    fn json(code: u8, payload: serde_json::Value) -> Self {
        Self {
            code,
            stdout: Some(payload.to_string()),
        }
    }

    fn text(code: u8, body: String) -> Self {
        Self {
            code,
            stdout: Some(body),
        }
    }
}

struct CommandError {
    code: u8,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl CommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_USAGE,
            stdout: None,
            stderr: Some(message.into()),
        }
    }

    fn config(error: ConfigError) -> Self {
        Self {
            code: EXIT_CONFIG,
            stdout: None,
            stderr: Some(format!("config error: {error}")),
        }
    }
}

struct ResolvedToken {
    token: String,
    source: TokenSource,
}

enum TokenSource {
    Env,
    Config,
}

impl TokenSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Config => "config",
        }
    }
}

struct AuthLoginArgs {
    output: OutputFormat,
    token_source: LoginTokenSource,
}

enum LoginTokenSource {
    Flag(String),
    Stdin,
}

enum OutputFormat {
    Text,
    Json,
}

struct AuthState {
    authenticated: bool,
    source: &'static str,
    username: Option<String>,
    logged_out: bool,
}

#[derive(Deserialize, Serialize)]
struct ConfigFile {
    token: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

enum ConfigError {
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

enum AuthError {
    InvalidToken,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}
