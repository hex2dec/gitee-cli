use crate::client::GiteeClient;
use crate::repo::RepoError;
use crate::utils::parse_api_error_message;
use serde::{Deserialize, Serialize};

pub enum PullRequestError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
    UnexpectedStatusWithMessage(u16, String),
}

pub struct CreatePullRequestComment<'a> {
    pub body: &'a str,
}

pub struct CreatePullRequest<'a> {
    pub title: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub body: Option<&'a str>,
}

pub struct MergePullRequest<'a> {
    pub merge_method: &'a str,
}

pub struct UpdatePullRequest<'a> {
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub state: Option<&'a str>,
    pub draft: Option<bool>,
}

#[derive(Serialize)]
struct CreatePullRequestPayload<'a> {
    access_token: &'a str,
    title: &'a str,
    head: &'a str,
    base: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

#[derive(Serialize)]
struct MergePullRequestPayload<'a> {
    merge_method: &'a str,
}

#[derive(Clone)]
pub struct PullRequestListFilters {
    pub state: Option<String>,
    pub author: Option<String>,
    pub assignee: Option<String>,
    pub base: Option<String>,
    pub head: Option<String>,
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct PullRequestResponse {
    pub number: u64,
    pub state: String,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub mergeable: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub merged_at: Option<String>,
    pub user: PullRequestUserResponse,
    pub head: PullRequestBranchResponse,
    pub base: PullRequestBranchResponse,
}

#[derive(Deserialize)]
pub struct PullRequestCommentResponse {
    pub id: u64,
    pub body: String,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub comment_type: Option<String>,
    pub user: PullRequestUserResponse,
}

#[derive(Deserialize)]
pub struct PullRequestMergeResponse {
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct PullRequestUserResponse {
    pub login: String,
}

#[derive(Deserialize)]
pub struct PullRequestBranchResponse {
    #[serde(rename = "ref")]
    pub branch: String,
    pub sha: String,
    #[serde(default)]
    pub repo: Option<PullRequestRepositoryResponse>,
}

#[derive(Deserialize)]
pub struct PullRequestRepositoryResponse {
    pub full_name: String,
}

impl GiteeClient {
    pub fn fetch_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        token: Option<&str>,
    ) -> Result<PullRequestResponse, PullRequestError> {
        let mut request = self.client.get(format!(
            "{}/v5/repos/{owner}/{repo}/pulls/{number}",
            self.base_url
        ));

        if let Some(token) = token {
            request = request.query(&[("access_token", token)]);
        }

        let response = request.send().map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(PullRequestError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(PullRequestError::NotFound);
        }

        Err(PullRequestError::UnexpectedStatus(
            response.status().as_u16(),
        ))
    }

    pub fn fetch_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        filters: &PullRequestListFilters,
        token: Option<&str>,
    ) -> Result<Vec<PullRequestResponse>, RepoError> {
        let mut request = self
            .client
            .get(format!("{}/v5/repos/{owner}/{repo}/pulls", self.base_url));

        let mut query = Vec::<(&str, String)>::new();

        if let Some(token) = token {
            query.push(("access_token", token.to_string()));
        }

        if let Some(state) = filters.state.as_deref() {
            query.push(("state", state.to_string()));
        }

        if let Some(author) = filters.author.as_deref() {
            query.push(("author", author.to_string()));
        }

        if let Some(assignee) = filters.assignee.as_deref() {
            query.push(("assignee", assignee.to_string()));
        }

        if let Some(base) = filters.base.as_deref() {
            query.push(("base", base.to_string()));
        }

        if let Some(head) = filters.head.as_deref() {
            query.push(("head", head.to_string()));
        }

        query.push(("per_page", filters.limit.to_string()));
        request = request.query(&query);

        let response = request.send().map_err(RepoError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<Vec<PullRequestResponse>>()
                .map_err(RepoError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(RepoError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(RepoError::NotFound);
        }

        Err(RepoError::UnexpectedStatus(response.status().as_u16()))
    }

    pub fn create_pull_request_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        token: &str,
        request: &CreatePullRequestComment<'_>,
    ) -> Result<PullRequestCommentResponse, PullRequestError> {
        let response = self
            .client
            .post(format!(
                "{}/v5/repos/{owner}/{repo}/pulls/{number}/comments",
                self.base_url
            ))
            .query(&[("access_token", token)])
            .form(&[("body", request.body)])
            .send()
            .map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<PullRequestCommentResponse>()
                .map_err(PullRequestError::Transport);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(PullRequestError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(PullRequestError::NotFound);
        }

        Err(PullRequestError::UnexpectedStatus(
            response.status().as_u16(),
        ))
    }

    pub fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        token: &str,
        request: &CreatePullRequest<'_>,
    ) -> Result<PullRequestResponse, PullRequestError> {
        let response = self
            .client
            .post(format!("{}/v5/repos/{owner}/{repo}/pulls", self.base_url))
            .json(&CreatePullRequestPayload {
                access_token: token,
                title: request.title,
                head: request.head,
                base: request.base,
                body: request.body,
            })
            .send()
            .map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if status == 401 {
            return Err(PullRequestError::InvalidToken);
        }

        if status == 404 {
            return Err(PullRequestError::NotFound);
        }

        if let Some(message) = error_message {
            return Err(PullRequestError::UnexpectedStatusWithMessage(
                status, message,
            ));
        }

        Err(PullRequestError::UnexpectedStatus(status))
    }

    pub fn update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        token: &str,
        request: &UpdatePullRequest<'_>,
    ) -> Result<PullRequestResponse, PullRequestError> {
        let mut form = vec![("access_token", token.to_string())];

        if let Some(title) = request.title {
            form.push(("title", title.to_string()));
        }

        if let Some(body) = request.body {
            form.push(("body", body.to_string()));
        }

        if let Some(state) = request.state {
            form.push(("state", state.to_string()));
        }

        if let Some(draft) = request.draft {
            form.push(("draft", draft.to_string()));
        }

        let response = self
            .client
            .patch(format!(
                "{}/v5/repos/{owner}/{repo}/pulls/{number}",
                self.base_url
            ))
            .query(&[("access_token", token)])
            .form(&form)
            .send()
            .map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if status == 401 {
            return Err(PullRequestError::InvalidToken);
        }

        if status == 404 {
            return Err(PullRequestError::NotFound);
        }

        if let Some(message) = error_message {
            return Err(PullRequestError::UnexpectedStatusWithMessage(
                status, message,
            ));
        }

        Err(PullRequestError::UnexpectedStatus(status))
    }

    pub fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        token: &str,
        request: &MergePullRequest<'_>,
    ) -> Result<PullRequestMergeResponse, PullRequestError> {
        let response = self
            .client
            .put(format!(
                "{}/v5/repos/{owner}/{repo}/pulls/{number}/merge",
                self.base_url
            ))
            .query(&[("access_token", token)])
            .json(&MergePullRequestPayload {
                merge_method: request.merge_method,
            })
            .send()
            .map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            return response
                .json::<PullRequestMergeResponse>()
                .map_err(PullRequestError::Transport);
        }

        let status = response.status().as_u16();
        let error_message = parse_api_error_message(response);

        if status == 401 {
            return Err(PullRequestError::InvalidToken);
        }

        if status == 404 {
            return Err(PullRequestError::NotFound);
        }

        if let Some(message) = error_message {
            return Err(PullRequestError::UnexpectedStatusWithMessage(
                status, message,
            ));
        }

        Err(PullRequestError::UnexpectedStatus(status))
    }
}
