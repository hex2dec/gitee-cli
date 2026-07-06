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

pub struct PullRequest {
    pub number: u64,
    pub state: String,
    pub title: String,
    pub body: Option<String>,
    pub author: String,
    pub repository: String,
    pub head: PullRequestBranch,
    pub base: PullRequestBranch,
    pub draft: bool,
    pub mergeable: Option<bool>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub merged_at: Option<String>,
}

pub struct PullRequestComment {
    pub id: u64,
    pub body: String,
    pub author: String,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub comment_type: String,
}

pub struct PullRequestMergeResult {
    pub sha: Option<String>,
    pub merged: bool,
    pub message: String,
}

pub struct PullRequestBranch {
    pub r#ref: String,
    pub sha: String,
    pub repository: String,
}

#[derive(Deserialize)]
struct PullRequestResponse {
    number: u64,
    state: String,
    title: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    mergeable: Option<bool>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    merged_at: Option<String>,
    user: PullRequestUserResponse,
    head: PullRequestBranchResponse,
    base: PullRequestBranchResponse,
}

#[derive(Deserialize)]
struct PullRequestCommentResponse {
    id: u64,
    body: String,
    html_url: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    comment_type: Option<String>,
    user: PullRequestUserResponse,
}

#[derive(Deserialize)]
struct PullRequestMergeResponse {
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    merged: bool,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct PullRequestUserResponse {
    login: String,
}

#[derive(Deserialize)]
struct PullRequestBranchResponse {
    #[serde(rename = "ref")]
    branch: String,
    sha: String,
    #[serde(default)]
    repo: Option<PullRequestRepositoryResponse>,
}

#[derive(Deserialize)]
struct PullRequestRepositoryResponse {
    full_name: String,
}

impl PullRequestResponse {
    fn into_pull_request(self, owner: &str, repo: &str) -> PullRequest {
        let repository = format!("{owner}/{repo}");

        PullRequest {
            number: self.number,
            state: self.state,
            title: self.title,
            body: self.body,
            author: self.user.login,
            repository: repository.clone(),
            head: self.head.into_pull_request_branch(),
            base: self.base.into_pull_request_branch_with_default(&repository),
            draft: self.draft,
            mergeable: self.mergeable,
            html_url: self.html_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
            merged_at: self.merged_at,
        }
    }
}

impl PullRequestBranchResponse {
    fn into_pull_request_branch(self) -> PullRequestBranch {
        self.into_pull_request_branch_with_default("")
    }

    fn into_pull_request_branch_with_default(self, default_repository: &str) -> PullRequestBranch {
        PullRequestBranch {
            r#ref: self.branch,
            sha: self.sha,
            repository: self
                .repo
                .map(|repo| repo.full_name)
                .unwrap_or_else(|| default_repository.to_string()),
        }
    }
}

impl PullRequestCommentResponse {
    fn into_pull_request_comment(self) -> PullRequestComment {
        PullRequestComment {
            id: self.id,
            body: self.body,
            author: self.user.login,
            html_url: self.html_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
            comment_type: self
                .comment_type
                .unwrap_or_else(|| "pr_comment".to_string()),
        }
    }
}

impl PullRequestMergeResponse {
    fn into_pull_request_merge_result(self) -> PullRequestMergeResult {
        PullRequestMergeResult {
            sha: self.sha,
            merged: self.merged,
            message: self.message.unwrap_or_default(),
        }
    }
}

impl GiteeClient {
    pub fn fetch_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        token: Option<&str>,
    ) -> Result<PullRequest, PullRequestError> {
        let mut request = self.client.get(format!(
            "{}/v5/repos/{owner}/{repo}/pulls/{number}",
            self.base_url
        ));

        if let Some(token) = token {
            request = request.query(&[("access_token", token)]);
        }

        let response = request.send().map_err(PullRequestError::Transport)?;

        if response.status().is_success() {
            let pull_request = response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport)?;
            return Ok(pull_request.into_pull_request(owner, repo));
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
    ) -> Result<Vec<PullRequest>, RepoError> {
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
            let pull_requests = response
                .json::<Vec<PullRequestResponse>>()
                .map_err(RepoError::Transport)?
                .into_iter()
                .map(|pull_request| pull_request.into_pull_request(owner, repo))
                .collect();
            return Ok(pull_requests);
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
    ) -> Result<PullRequestComment, PullRequestError> {
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
            let comment = response
                .json::<PullRequestCommentResponse>()
                .map_err(PullRequestError::Transport)?;
            return Ok(comment.into_pull_request_comment());
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
    ) -> Result<PullRequest, PullRequestError> {
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
            let pull_request = response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport)?;
            return Ok(pull_request.into_pull_request(owner, repo));
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
    ) -> Result<PullRequest, PullRequestError> {
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
            let pull_request = response
                .json::<PullRequestResponse>()
                .map_err(PullRequestError::Transport)?;
            return Ok(pull_request.into_pull_request(owner, repo));
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
    ) -> Result<PullRequestMergeResult, PullRequestError> {
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
            let result = response
                .json::<PullRequestMergeResponse>()
                .map_err(PullRequestError::Transport)?;
            return Ok(result.into_pull_request_merge_result());
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
