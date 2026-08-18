use crate::client::GiteeClient;
use crate::utils::{parse_api_error_message, response_indicates_invalid_token};
use serde::Deserialize;

#[derive(Debug)]
pub enum RepoError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

#[derive(Deserialize)]
pub struct RepositoryResponse {
    pub full_name: String,
    #[serde(default)]
    pub human_name: Option<String>,
    pub path: String,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub ssh_url: Option<String>,
    #[serde(default)]
    pub clone_url: Option<String>,
    pub fork: bool,
    pub default_branch: String,
}

impl RepositoryResponse {
    fn matches_slug_or_human_name(&self, owner: &str, repo: &str) -> bool {
        self.full_name == format!("{owner}/{repo}")
            || self.human_name.as_deref() == Some(&format!("{owner}/{repo}"))
    }
}

impl GiteeClient {
    pub fn fetch_repository(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
    ) -> Result<RepositoryResponse, RepoError> {
        let request = self.with_optional_auth(
            self.client
                .get(format!("{}/v5/repos/{owner}/{repo}", self.base_url)),
            token,
        );

        let response = request.send().map_err(RepoError::Transport)?;

        if response.status().is_success() {
            let repository = response
                .json::<RepositoryResponse>()
                .map_err(RepoError::Transport)?;
            return Ok(repository);
        }

        let status = response.status().as_u16();

        if status == 404 {
            return Err(RepoError::NotFound);
        }

        let error_message = parse_api_error_message(response);

        if response_indicates_invalid_token(status, error_message.as_deref()) {
            return Err(RepoError::InvalidToken);
        }

        Err(RepoError::UnexpectedStatus(status))
    }

    pub fn find_repository_by_human_name(
        &self,
        owner: &str,
        repo: &str,
        token: &str,
    ) -> Result<Option<RepositoryResponse>, RepoError> {
        let response = self
            .with_auth(
                self.client.get(format!("{}/v5/user/repos", self.base_url)),
                token,
            )
            .send()
            .map_err(RepoError::Transport)?;

        if response.status().is_success() {
            let repository = response
                .json::<Vec<RepositoryResponse>>()
                .map_err(RepoError::Transport)?
                .into_iter()
                .find(|candidate| candidate.matches_slug_or_human_name(owner, repo));
            return Ok(repository);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if response_indicates_invalid_token(status, error_message.as_deref()) {
            return Err(RepoError::InvalidToken);
        }

        Err(RepoError::UnexpectedStatus(status))
    }
}
