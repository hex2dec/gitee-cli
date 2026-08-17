use crate::client::GiteeClient;
use crate::utils::parse_api_error_message;
use serde::Deserialize;

#[derive(Debug)]
pub enum IssueError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
    UnexpectedStatusWithMessage(u16, String),
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

        if let Some(token) = token {
            query.push(("access_token", token.to_string()));
        }

        let response = self
            .client
            .get(format!("{}/v5/repos/{owner}/{repo}/issues", self.base_url))
            .query(&query)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<Vec<IssueResponse>>()
                .map_err(IssueError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(IssueError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(IssueError::NotFound);
        }

        Err(IssueError::UnexpectedStatus(response.status().as_u16()))
    }

    pub fn fetch_issue(
        &self,
        owner: &str,
        repo: &str,
        number: &str,
        token: Option<&str>,
    ) -> Result<IssueResponse, IssueError> {
        let mut request = self.client.get(format!(
            "{}/v5/repos/{owner}/{repo}/issues/{number}",
            self.base_url
        ));

        if let Some(token) = token {
            request = request.query(&[("access_token", token)]);
        }

        let response = request.send().map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(IssueError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(IssueError::NotFound);
        }

        Err(IssueError::UnexpectedStatus(response.status().as_u16()))
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
        let mut query = vec![
            ("page", page.to_string()),
            ("per_page", per_page.to_string()),
        ];

        if let Some(token) = token {
            query.push(("access_token", token.to_string()));
        }

        let response = self
            .client
            .get(format!(
                "{}/v5/repos/{owner}/{repo}/issues/{number}/comments",
                self.base_url
            ))
            .query(&query)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<Vec<IssueCommentResponse>>()
                .map_err(IssueError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(IssueError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(IssueError::NotFound);
        }

        Err(IssueError::UnexpectedStatus(response.status().as_u16()))
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
            .client
            .post(format!(
                "{}/v5/repos/{owner}/{repo}/issues/{number}/comments",
                self.base_url
            ))
            .query(&[("access_token", token)])
            .form(&[("body", body)])
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueCommentResponse>()
                .map_err(IssueError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(IssueError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(IssueError::NotFound);
        }

        Err(IssueError::UnexpectedStatus(response.status().as_u16()))
    }

    pub fn create_issue(
        &self,
        owner: &str,
        token: &str,
        request: &CreateIssue<'_>,
    ) -> Result<IssueResponse, IssueError> {
        let mut form = vec![
            ("access_token", token.to_string()),
            ("repo", request.repo.to_string()),
            ("title", request.title.to_string()),
        ];

        if let Some(body) = request.body {
            form.push(("body", body.to_string()));
        }

        let response = self
            .client
            .post(format!("{}/v5/repos/{owner}/issues", self.base_url))
            .form(&form)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if status == 401 {
            return Err(IssueError::InvalidToken);
        }

        if status == 404 {
            return Err(IssueError::NotFound);
        }

        if let Some(message) = error_message {
            return Err(IssueError::UnexpectedStatusWithMessage(status, message));
        }

        Err(IssueError::UnexpectedStatus(status))
    }

    pub fn update_issue(
        &self,
        owner: &str,
        number: &str,
        token: &str,
        request: &UpdateIssue<'_>,
    ) -> Result<IssueResponse, IssueError> {
        let mut form = vec![("repo", request.repo.to_string())];

        if let Some(title) = request.title {
            form.push(("title", title.to_string()));
        }

        if let Some(body) = request.body {
            form.push(("body", body.to_string()));
        }

        if let Some(state) = request.state {
            form.push(("state", state.to_string()));
        }

        let response = self
            .client
            .patch(format!(
                "{}/v5/repos/{owner}/issues/{number}",
                self.base_url
            ))
            .query(&[("access_token", token)])
            .form(&form)
            .send()
            .map_err(IssueError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if status == 401 {
            return Err(IssueError::InvalidToken);
        }

        if status == 404 {
            return Err(IssueError::NotFound);
        }

        if let Some(message) = error_message {
            return Err(IssueError::UnexpectedStatusWithMessage(status, message));
        }

        Err(IssueError::UnexpectedStatus(status))
    }
}
