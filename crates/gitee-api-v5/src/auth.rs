use crate::client::GiteeClient;
use crate::utils::{parse_api_error_message, response_indicates_invalid_token};
use serde::Deserialize;

#[derive(Debug)]
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
            .with_auth(self.client.get(format!("{}/v5/user", self.base_url)), token)
            .send()
            .map_err(AuthError::Transport)?;

        if response.status().is_success() {
            let user = response
                .json::<UserResponse>()
                .map_err(AuthError::Transport)?;
            return Ok(user.login);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if response_indicates_invalid_token(status, error_message.as_deref()) {
            return Err(AuthError::InvalidToken);
        }

        Err(AuthError::UnexpectedStatus(status))
    }
}
