use crate::client::GiteeClient;
use crate::utils::ApiResponseError;
use serde::Deserialize;

#[derive(Debug)]
pub enum RepoError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

impl ApiResponseError for RepoError {
    fn invalid_token() -> Self {
        Self::InvalidToken
    }

    fn not_found() -> Self {
        Self::NotFound
    }

    fn unexpected_status(status: u16) -> Self {
        Self::UnexpectedStatus(status)
    }
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

        Err(RepoError::from_response_with_not_found_first(response))
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

        Err(RepoError::from_response_without_not_found(response))
    }
}
