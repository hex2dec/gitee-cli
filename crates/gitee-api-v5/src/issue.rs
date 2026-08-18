use crate::client::GiteeClient;
use crate::utils::ApiResponseError;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum IssueError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
    UnexpectedStatusWithMessage(u16, String),
}

impl ApiResponseError for IssueError {
    fn invalid_token() -> Self {
        Self::InvalidToken
    }

    fn not_found() -> Self {
        Self::NotFound
    }

    fn unexpected_status(status: u16) -> Self {
        Self::UnexpectedStatus(status)
    }

    fn unexpected_status_with_message(status: u16, message: String) -> Self {
        Self::UnexpectedStatusWithMessage(status, message)
    }
}

pub struct CreateIssue<'a> {
    pub repo: &'a str,
    pub title: &'a str,
    pub body: Option<&'a str>,
}

pub struct UpdateIssue<'a> {
    pub repo: &'a str,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub state: Option<&'a str>,
}

pub struct IssueListOptions<'a> {
    pub state: &'a str,
    pub search: Option<&'a str>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Serialize)]
struct CreateIssuePayload<'a> {
    repo: &'a str,
    title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

#[derive(Serialize)]
struct IssueCommentPayload<'a> {
    body: &'a str,
}

#[derive(Serialize)]
struct UpdateIssuePayload<'a> {
    repo: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct IssueResponse {
    pub number: String,
    pub title: String,
    pub state: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub comments: u64,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub user: Option<IssueUserResponse>,
}

#[derive(Deserialize)]
pub struct IssueUserResponse {
    pub login: String,
}

#[derive(Deserialize)]
pub struct IssueCommentResponse {
    pub id: u64,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub user: Option<IssueUserResponse>,
}

impl GiteeClient {
    pub fn list_repository_issues(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
        options: IssueListOptions<'_>,
    ) -> Result<Vec<IssueResponse>, IssueError> {
        let mut query = vec![
            ("state", options.state.to_string()),
            ("page", options.page.to_string()),
            ("per_page", options.per_page.to_string()),
        ];

        if let Some(search) = options.search {
            query.push(("q", search.to_string()));
        }

        let response = self
            .with_optional_auth(
                self.client
                    .get(format!("{}/v5/repos/{owner}/{repo}/issues", self.base_url)),
                token,
            )
            .query(&query)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<Vec<IssueResponse>>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_not_found_first(response))
    }

    pub fn fetch_issue(
        &self,
        owner: &str,
        repo: &str,
        number: &str,
        token: Option<&str>,
    ) -> Result<IssueResponse, IssueError> {
        let request = self.with_optional_auth(
            self.client.get(format!(
                "{}/v5/repos/{owner}/{repo}/issues/{number}",
                self.base_url
            )),
            token,
        );

        let response = request.send().map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_not_found_first(response))
    }

    pub fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        number: &str,
        token: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<IssueCommentResponse>, IssueError> {
        let query = vec![
            ("page", page.to_string()),
            ("per_page", per_page.to_string()),
        ];

        let response = self
            .with_optional_auth(
                self.client.get(format!(
                    "{}/v5/repos/{owner}/{repo}/issues/{number}/comments",
                    self.base_url
                )),
                token,
            )
            .query(&query)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<Vec<IssueCommentResponse>>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_not_found_first(response))
    }

    pub fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        number: &str,
        token: &str,
        body: &str,
    ) -> Result<IssueCommentResponse, IssueError> {
        let response = self
            .with_auth(
                self.client.post(format!(
                    "{}/v5/repos/{owner}/{repo}/issues/{number}/comments",
                    self.base_url
                )),
                token,
            )
            .json(&IssueCommentPayload { body })
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueCommentResponse>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_not_found_first(response))
    }

    pub fn create_issue(
        &self,
        owner: &str,
        token: &str,
        request: &CreateIssue<'_>,
    ) -> Result<IssueResponse, IssueError> {
        let response = self
            .with_auth(
                self.client
                    .post(format!("{}/v5/repos/{owner}/issues", self.base_url)),
                token,
            )
            .json(&CreateIssuePayload {
                repo: request.repo,
                title: request.title,
                body: request.body,
            })
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_token_first(response))
    }

    pub fn update_issue(
        &self,
        owner: &str,
        number: &str,
        token: &str,
        request: &UpdateIssue<'_>,
    ) -> Result<IssueResponse, IssueError> {
        let response = self
            .with_auth(
                self.client.patch(format!(
                    "{}/v5/repos/{owner}/issues/{number}",
                    self.base_url
                )),
                token,
            )
            .json(&UpdateIssuePayload {
                repo: request.repo,
                title: request.title,
                body: request.body,
                state: request.state,
            })
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        Err(IssueError::from_response_with_token_first(response))
    }
}
