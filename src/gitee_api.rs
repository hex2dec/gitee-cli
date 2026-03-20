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

    pub fn fetch_repository(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
    ) -> Result<Repository, RepoError> {
        let mut request = self
            .client
            .get(format!("{}/v5/repos/{owner}/{repo}", self.base_url));

        if let Some(token) = token {
            request = request.query(&[("access_token", token)]);
        }

        let response = request.send().map_err(RepoError::Transport)?;

        if response.status().is_success() {
            let repository = response
                .json::<RepositoryResponse>()
                .map_err(RepoError::Transport)?;
            return Ok(repository.into_repository());
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(RepoError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(RepoError::NotFound);
        }

        Err(RepoError::UnexpectedStatus(response.status().as_u16()))
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

pub enum RepoError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

pub struct Repository {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub fork: bool,
    pub default_branch: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct RepositoryResponse {
    full_name: String,
    path: String,
    html_url: String,
    ssh_url: String,
    clone_url: String,
    fork: bool,
    default_branch: String,
}

impl RepositoryResponse {
    fn into_repository(self) -> Repository {
        let owner = self
            .full_name
            .split_once('/')
            .map(|(owner, _)| owner.to_string())
            .unwrap_or_default();

        Repository {
            owner,
            name: self.path,
            full_name: self.full_name,
            html_url: self.html_url,
            ssh_url: self.ssh_url,
            clone_url: self.clone_url,
            fork: self.fork,
            default_branch: self.default_branch,
        }
    }
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
