use crate::client::GiteeClient;
use serde::Deserialize;

pub enum AuthError {
    InvalidToken,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

impl GiteeClient {
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
