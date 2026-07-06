use crate::client::GiteeClient;
use crate::utils::parse_api_error_message;
use serde::Deserialize;

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

impl GiteeClient {
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

    pub fn create_issue(
        &self,
        owner: &str,
        token: &str,
        request: &CreateIssue<'_>,
    ) -> Result<Issue, IssueError> {
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
            let issue = response
                .json::<IssueResponse>()
                .map_err(IssueError::Transport)?
                .into_issue();
            return Ok(issue);
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
