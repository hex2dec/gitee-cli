use std::env;

use reqwest::blocking::Client;
use serde::Deserialize;

pub struct GiteeClient {
    client: Client,
    base_url: String,
}

pub struct CreatePullRequestComment<'a> {
    pub body: &'a str,
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

    pub fn list_repository_issues(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
        options: IssueListOptions<'_>,
    ) -> Result<Vec<Issue>, IssueError> {
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
            let issues = response
                .json::<Vec<IssueResponse>>()
                .map_err(IssueError::Transport)?
                .into_iter()
                .map(IssueResponse::into_issue)
                .collect();
            return Ok(issues);
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
    ) -> Result<Issue, IssueError> {
        let mut request = self.client.get(format!(
            "{}/v5/repos/{owner}/{repo}/issues/{number}",
            self.base_url
        ));

        if let Some(token) = token {
            request = request.query(&[("access_token", token)]);
        }

        let response = request.send().map_err(IssueError::Transport)?;

        if response.status().is_success() {
            let issue = response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport)?
                .into_issue();
            return Ok(issue);
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
    ) -> Result<Vec<IssueComment>, IssueError> {
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
            let comments = response
                .json::<Vec<IssueCommentResponse>>()
                .map_err(IssueError::Transport)?
                .into_iter()
                .map(IssueCommentResponse::into_issue_comment)
                .collect();
            return Ok(comments);
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
    ) -> Result<IssueComment, IssueError> {
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
            let comment = response
                .json::<IssueCommentResponse>()
                .map_err(IssueError::Transport)?
                .into_issue_comment();
            return Ok(comment);
        }

        if matches!(response.status().as_u16(), 400 | 401) {
            return Err(IssueError::InvalidToken);
        }

        if response.status().as_u16() == 404 {
            return Err(IssueError::NotFound);
        }

        Err(IssueError::UnexpectedStatus(response.status().as_u16()))
    }

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

pub enum IssueError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
}

pub enum PullRequestError {
    InvalidToken,
    NotFound,
    Transport(reqwest::Error),
    UnexpectedStatus(u16),
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

pub struct IssueListOptions<'a> {
    pub state: &'a str,
    pub search: Option<&'a str>,
    pub page: u32,
    pub per_page: u32,
}

pub struct Issue {
    pub number: String,
    pub title: String,
    pub state: String,
    pub body: String,
    pub author: String,
    pub comments: u64,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct IssueComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
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

pub struct PullRequestBranch {
    pub r#ref: String,
    pub sha: String,
    pub repository: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
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

#[derive(Deserialize)]
struct IssueResponse {
    number: String,
    title: String,
    state: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    comments: u64,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    user: Option<IssueUserResponse>,
}

#[derive(Deserialize)]
struct IssueUserResponse {
    login: String,
}

#[derive(Deserialize)]
struct IssueCommentResponse {
    id: u64,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    user: Option<IssueUserResponse>,
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

impl RepositoryResponse {
    fn into_repository(self) -> Repository {
        let full_name = self.full_name;
        let owner = full_name
            .split_once('/')
            .map(|(owner, _)| owner.to_string())
            .unwrap_or_default();
        let html_url = self
            .html_url
            .map(|value| normalize_html_url(&value, &full_name))
            .unwrap_or_else(|| format!("https://gitee.com/{full_name}"));
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

impl IssueResponse {
    fn into_issue(self) -> Issue {
        Issue {
            number: self.number,
            title: self.title,
            state: self.state,
            body: self.body.unwrap_or_default(),
            author: self.user.map(|user| user.login).unwrap_or_default(),
            comments: self.comments,
            html_url: self.html_url.unwrap_or_default(),
            created_at: self.created_at.unwrap_or_default(),
            updated_at: self.updated_at.unwrap_or_default(),
        }
    }
}

impl IssueCommentResponse {
    fn into_issue_comment(self) -> IssueComment {
        IssueComment {
            id: self.id,
            author: self.user.map(|user| user.login).unwrap_or_default(),
            body: self.body.unwrap_or_default(),
            created_at: self.created_at.unwrap_or_default(),
            updated_at: self.updated_at.unwrap_or_default(),
        }
    }
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

fn normalize_html_url(value: &str, full_name: &str) -> String {
    if value.is_empty() {
        return format!("https://gitee.com/{full_name}");
    }

    value.trim_end_matches(".git").to_string()
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
