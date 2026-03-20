pub const EXIT_OK: u8 = 0;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_AUTH: u8 = 3;
pub const EXIT_CONFIG: u8 = 4;
pub const EXIT_REMOTE: u8 = 5;
pub const EXIT_NOT_FOUND: u8 = 6;
pub const EXIT_GIT: u8 = 7;

pub struct CommandOutcome {
    pub code: u8,
    pub stdout: Option<String>,
}

impl CommandOutcome {
    pub fn json(code: u8, payload: serde_json::Value) -> Self {
        Self {
            code,
            stdout: Some(payload.to_string()),
        }
    }

    pub fn text(code: u8, body: String) -> Self {
        Self {
            code,
            stdout: Some(body),
        }
    }
}

pub struct CommandError {
    pub code: u8,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CommandError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_USAGE,
            stdout: None,
            stderr: Some(message.into()),
        }
    }

    pub fn config(error: impl std::fmt::Display) -> Self {
        Self {
            code: EXIT_CONFIG,
            stdout: None,
            stderr: Some(format!("config error: {error}")),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_NOT_FOUND,
            stdout: None,
            stderr: Some(message.into()),
        }
    }

    pub fn git(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_GIT,
            stdout: None,
            stderr: Some(message.into()),
        }
    }
}

#[derive(Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
}
