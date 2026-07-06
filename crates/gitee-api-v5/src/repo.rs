use crate::client::GiteeClient;
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
            return Ok(repository);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(RepoError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(RepoError::NotFound);
        }

        Err(RepoError::UnexpectedStatus(response.status().as_u16()))
    }

    pub fn find_repository_by_human_name(
        &self,
        owner: &str,
        repo: &str,
        token: &str,
    ) -> Result<Option<RepositoryResponse>, RepoError> {
        let response = self
            .client
            .get(format!("{}/v5/user/repos", self.base_url))
            .query(&[("access_token", token)])
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

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(RepoError::InvalidToken);
        }

        Err(RepoError::UnexpectedStatus(response.status().as_u16()))
    }
}
