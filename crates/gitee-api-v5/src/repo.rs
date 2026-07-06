use crate::client::GiteeClient;
use serde::Deserialize;

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
struct RepositoryResponse {
    full_name: String,
    #[serde(default)]
    human_name: Option<String>,
    path: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    ssh_url: Option<String>,
    #[serde(default)]
    clone_url: Option<String>,
    fork: bool,
    default_branch: String,
}

impl RepositoryResponse {
    fn into_repository(self) -> Repository {
        let full_name = self.full_name;
        let owner = full_name
            .split_once('/')
            .map(|(owner, _)| owner.to_string())
            .unwrap_or_default();
        let html_url = self.html_url.unwrap_or_default();
        let ssh_url = self
            .ssh_url
            .unwrap_or_else(|| format!("git@gitee.com:{full_name}.git"));
        let clone_url = self
            .clone_url
            .unwrap_or_else(|| format!("https://gitee.com/{full_name}.git"));

        Repository {
            owner,
            name: self.path,
            full_name,
            html_url,
            ssh_url,
            clone_url,
            fork: self.fork,
            default_branch: self.default_branch,
        }
    }

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

    pub fn find_repository_by_human_name(
        &self,
        owner: &str,
        repo: &str,
        token: &str,
    ) -> Result<Option<Repository>, RepoError> {
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
                .find(|candidate| candidate.matches_slug_or_human_name(owner, repo))
                .map(RepositoryResponse::into_repository);
            return Ok(repository);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(RepoError::InvalidToken);
        }

        Err(RepoError::UnexpectedStatus(response.status().as_u16()))
    }
}
