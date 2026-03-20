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
            base_url: resolve_base_url(env::var("GITEE_BASE_URL").ok()),
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

fn resolve_base_url(value: Option<String>) -> String {
    value
        .unwrap_or_else(|| "https://gitee.com/api".to_string())
        .trim_end_matches('/')
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::resolve_base_url;

    #[test]
    fn defaults_to_gitee_api_base_path() {
        assert_eq!(resolve_base_url(None), "https://gitee.com/api");
    }

    #[test]
    fn trims_trailing_slash_from_custom_base_url() {
        assert_eq!(
            resolve_base_url(Some("http://127.0.0.1:1234/".to_string())),
            "http://127.0.0.1:1234"
        );
    }
}
