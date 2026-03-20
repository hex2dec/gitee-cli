use std::env;

use reqwest::blocking::Client;
use serde::Deserialize;

pub struct GiteeClient {
    client: Client,
    base_url: String,
}

impl GiteeClient {
    pub fn from_env() -> Self {
        Self {
            client: Client::new(),
            base_url: env::var("GITEE_BASE_URL")
                .unwrap_or_else(|_| "https://gitee.com".to_string()),
        }
    }

    pub fn fetch_current_user(&self, token: &str) -> Result<String, AuthError> {
        let response = self
            .client
            .get(format!("{}/v5/user", self.base_url))
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
}

pub enum AuthError {
    InvalidToken,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}
