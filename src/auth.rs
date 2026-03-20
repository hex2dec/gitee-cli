use std::io::{self, Read};

use serde_json::json;

use crate::command::{CommandError, CommandOutcome, EXIT_AUTH, EXIT_OK, EXIT_REMOTE, OutputFormat};
use crate::config::ConfigStore;
use crate::gitee_api::{AuthError, GiteeClient};

pub struct AuthService {
    config: ConfigStore,
    client: GiteeClient,
}

impl AuthService {
    pub fn from_env() -> Self {
        Self {
            config: ConfigStore::from_env(),
            client: GiteeClient::from_env(),
        }
    }

    pub fn status(&self, output: OutputFormat) -> Result<CommandOutcome, CommandError> {
        let Some(credentials) = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
        else {
            return Ok(self.render_state(
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

        let username = self
            .client
            .fetch_current_user(&credentials.token)
            .map_err(map_auth_error)?;

        Ok(self.render_state(
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

    pub fn login(&self, request: LoginRequest) -> Result<CommandOutcome, CommandError> {
        let token = match request.token_source {
            LoginTokenSource::Flag(token) => token,
            LoginTokenSource::Stdin => read_token_from_stdin().map_err(CommandError::usage)?,
        };

        let username = self
            .client
            .fetch_current_user(&token)
            .map_err(map_auth_error)?;
        self.config
            .save_token(&token)
            .map_err(CommandError::config)?;

        Ok(self.render_state(
            request.output,
            EXIT_OK,
            AuthState {
                authenticated: true,
                source: "config",
                username: Some(username),
                logged_out: false,
            },
        ))
    }

    pub fn logout(&self, output: OutputFormat) -> Result<CommandOutcome, CommandError> {
        self.config.clear_token().map_err(CommandError::config)?;

        Ok(self.render_state(
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

    fn render_state(&self, output: OutputFormat, code: u8, state: AuthState) -> CommandOutcome {
        match output {
            OutputFormat::Json => CommandOutcome::json(
                code,
                json!({
                    "authenticated": state.authenticated,
                    "source": state.source,
                    "username": state.username,
                    "logged_out": state.logged_out,
                    "config_path": self.config.config_path(),
                }),
            ),
            OutputFormat::Text => CommandOutcome::text(code, render_auth_text(&state)),
        }
    }
}

pub struct LoginRequest {
    pub output: OutputFormat,
    pub token_source: LoginTokenSource,
}

pub enum LoginTokenSource {
    Flag(String),
    Stdin,
}

struct AuthState {
    authenticated: bool,
    source: &'static str,
    username: Option<String>,
    logged_out: bool,
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
