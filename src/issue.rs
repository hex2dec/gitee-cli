use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use serde_json::json;

use crate::command::{CommandError, CommandOutcome, EXIT_AUTH, EXIT_OK, EXIT_REMOTE, OutputFormat};
use crate::config::ConfigStore;
use crate::gitee_api::{
    CreateIssue, GiteeClient, Issue, IssueComment, IssueError, IssueListOptions,
};
use crate::repo_context::infer_repo_context;

pub struct IssueService {
    config: ConfigStore,
    client: GiteeClient,
}

impl IssueService {
    pub fn from_env() -> Self {
        Self {
            config: ConfigStore::from_env(),
            client: GiteeClient::from_env(),
        }
    }

    pub fn list(&self, request: IssueListRequest) -> Result<CommandOutcome, CommandError> {
        let resolved = resolve_issue_repo(request.repo.as_deref())?;

        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);

        let issues = self
            .client
            .list_repository_issues(
                &resolved.owner,
                &resolved.name,
                token.as_deref(),
                IssueListOptions {
                    state: request.state.as_query_value(),
                    search: request.search.as_deref(),
                    page: request.page,
                    per_page: request.per_page,
                },
            )
            .map_err(map_issue_list_error)?;

        Ok(render_issue_list(
            request.output,
            IssueListView {
                source: resolved.source,
                owner: resolved.owner,
                name: resolved.name,
                state: request.state.as_query_value(),
                search: request.search,
                page: request.page,
                per_page: request.per_page,
                issues,
            },
        ))
    }

    pub fn view(&self, request: IssueViewRequest) -> Result<CommandOutcome, CommandError> {
        let resolved = resolve_issue_repo(request.repo.as_deref())?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);
        let issue = self
            .client
            .fetch_issue(
                &resolved.owner,
                &resolved.name,
                &request.number,
                token.as_deref(),
            )
            .map_err(|error| map_issue_error(error, "issue not found"))?;
        let comments = if request.comments {
            Some(
                self.client
                    .list_issue_comments(
                        &resolved.owner,
                        &resolved.name,
                        &request.number,
                        token.as_deref(),
                        request.page,
                        request.per_page,
                    )
                    .map_err(|error| map_issue_error(error, "issue not found"))?,
            )
        } else {
            None
        };

        Ok(render_issue_view(
            request.output,
            IssueView {
                source: resolved.source,
                owner: resolved.owner,
                name: resolved.name,
                issue,
                comments_included: request.comments,
                comments_page: request.comments.then_some(request.page),
                comments_per_page: request.comments.then_some(request.per_page),
                comments,
            },
        ))
    }

    pub fn create(&self, request: IssueCreateRequest) -> Result<CommandOutcome, CommandError> {
        let resolved = resolve_issue_repo(request.repo.as_deref())?;
        let body = read_optional_issue_body(request.body)?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .ok_or_else(|| CommandError {
                code: EXIT_AUTH,
                stdout: None,
                stderr: Some("authentication required for issue create".to_string()),
            })?
            .token;
        let issue = self
            .client
            .create_issue(
                &resolved.owner,
                &token,
                &CreateIssue {
                    repo: &resolved.name,
                    title: &request.title,
                    body: body.as_deref(),
                },
            )
            .map_err(map_issue_create_error)?;

        Ok(render_issue_create(
            request.output,
            IssueCreateView {
                source: resolved.source,
                owner: resolved.owner,
                name: resolved.name,
                issue,
            },
        ))
    }

    pub fn comment(&self, request: IssueCommentRequest) -> Result<CommandOutcome, CommandError> {
        let resolved = resolve_issue_repo(request.repo.as_deref())?;
        let body = read_required_issue_body(
            request.body,
            "failed to read comment body from stdin",
            "failed to read comment body file",
            "comment body cannot be empty",
        )?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .ok_or_else(|| CommandError {
                code: EXIT_AUTH,
                stdout: None,
                stderr: Some("authentication required for issue comment".to_string()),
            })?
            .token;
        let comment = self
            .client
            .create_issue_comment(
                &resolved.owner,
                &resolved.name,
                &request.number,
                &token,
                &body,
            )
            .map_err(|error| map_issue_error(error, "issue not found"))?;

        Ok(render_issue_comment(
            request.output,
            IssueCommentView {
                source: resolved.source,
                owner: resolved.owner,
                name: resolved.name,
                number: request.number,
                comment,
            },
        ))
    }
}

pub struct IssueListRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub state: IssueStateFilter,
    pub search: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

pub struct IssueViewRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub number: String,
    pub comments: bool,
    pub page: u32,
    pub per_page: u32,
}

pub struct IssueCreateRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub title: String,
    pub body: Option<IssueBodySource>,
}

pub struct IssueCommentRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub number: String,
    pub body: IssueBodySource,
}

pub enum IssueBodySource {
    Inline(String),
    File(PathBuf),
}

#[derive(Clone, Copy)]
pub enum IssueStateFilter {
    Open,
    Closed,
    All,
}

impl IssueStateFilter {
    pub fn parse(value: &str) -> Result<Self, CommandError> {
        match value {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "all" => Ok(Self::All),
            _ => Err(CommandError::usage(
                "invalid value for --state: expected open, closed, or all",
            )),
        }
    }

    pub fn as_query_value(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

struct IssueListView {
    source: &'static str,
    owner: String,
    name: String,
    state: &'static str,
    search: Option<String>,
    page: u32,
    per_page: u32,
    issues: Vec<Issue>,
}

struct IssueView {
    source: &'static str,
    owner: String,
    name: String,
    issue: Issue,
    comments_included: bool,
    comments_page: Option<u32>,
    comments_per_page: Option<u32>,
    comments: Option<Vec<IssueComment>>,
}

struct IssueCreateView {
    source: &'static str,
    owner: String,
    name: String,
    issue: Issue,
}

struct IssueCommentView {
    source: &'static str,
    owner: String,
    name: String,
    number: String,
    comment: IssueComment,
}

struct ResolvedIssueRepo {
    owner: String,
    name: String,
    source: &'static str,
}

struct RepoSlug {
    owner: String,
    name: String,
}

impl RepoSlug {
    fn parse(value: &str) -> Result<Self, CommandError> {
        let Some((owner, name)) = value.split_once('/') else {
            return Err(CommandError::usage(
                "invalid value for --repo: expected owner/repo",
            ));
        };

        if owner.is_empty() || name.is_empty() || name.contains('/') {
            return Err(CommandError::usage(
                "invalid value for --repo: expected owner/repo",
            ));
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }
}

fn render_issue_list(output: OutputFormat, view: IssueListView) -> CommandOutcome {
    match output {
        OutputFormat::Json { fields } => CommandOutcome::json(
            EXIT_OK,
            match fields {
                Some(fields) => serde_json::Value::Array(
                    view.issues
                        .iter()
                        .map(|issue| issue_selected_json(issue, &fields))
                        .collect(),
                ),
                None => json!({
                    "source": view.source,
                    "owner": view.owner,
                    "name": view.name,
                    "state": view.state,
                    "search": view.search,
                    "page": view.page,
                    "per_page": view.per_page,
                    "issues": view.issues.iter().map(issue_summary_json).collect::<Vec<_>>(),
                }),
            },
        ),
        OutputFormat::Text => {
            let mut lines = vec![
                format!("{}/{} issues", view.owner, view.name),
                format!("state: {}", view.state),
                format!("search: {}", view.search.as_deref().unwrap_or("(none)")),
                format!("page: {}", view.page),
                format!("per page: {}", view.per_page),
                format!("source: {}", view.source),
            ];

            if view.issues.is_empty() {
                lines.push("(no issues)".to_string());
            } else {
                lines.extend(view.issues.into_iter().map(|issue| {
                    format!(
                        "{} | {} | {} | comments: {} | {}",
                        issue.number, issue.state, issue.author, issue.comments, issue.title
                    )
                }));
            }

            CommandOutcome::text(EXIT_OK, lines.join("\n"))
        }
    }
}

fn issue_summary_json(issue: &Issue) -> serde_json::Value {
    json!({
        "number": issue.number,
        "title": issue.title,
        "state": issue.state,
        "author": issue.author,
        "comments": issue.comments,
        "html_url": issue.html_url,
        "created_at": issue.created_at,
        "updated_at": issue.updated_at,
    })
}

fn issue_selected_json(issue: &Issue, fields: &[String]) -> serde_json::Value {
    let mut selected = serde_json::Map::with_capacity(fields.len());

    for field in fields {
        let value = match field.as_str() {
            "number" => json!(issue.number),
            "title" => json!(issue.title),
            "url" => json!(issue.html_url),
            _ => unreachable!("unsupported issue json field"),
        };

        selected.insert(field.clone(), value);
    }

    serde_json::Value::Object(selected)
}

fn render_issue_view(output: OutputFormat, view: IssueView) -> CommandOutcome {
    match output {
        OutputFormat::Json { fields } => CommandOutcome::json(
            EXIT_OK,
            match fields {
                Some(fields) => issue_view_selected_json(&view, &fields),
                None => issue_view_json(&view),
            },
        ),
        OutputFormat::Text => {
            let mut lines = vec![
                format!("{}/{}#{}", view.owner, view.name, view.issue.number),
                format!("title: {}", view.issue.title),
                format!("state: {}", view.issue.state),
                format!("author: {}", view.issue.author),
                format!("comments: {}", view.issue.comments),
                format!("created at: {}", view.issue.created_at),
                format!("updated at: {}", view.issue.updated_at),
                format!("html url: {}", view.issue.html_url),
                format!("source: {}", view.source),
                format!("comments included: {}", view.comments_included),
            ];

            if let (Some(page), Some(per_page)) = (view.comments_page, view.comments_per_page) {
                lines.push(format!("comments page: {}", page));
                lines.push(format!("comments per page: {}", per_page));
            }

            lines.push("body:".to_string());
            lines.push(view.issue.body);

            if let Some(comments) = view.comments {
                if comments.is_empty() {
                    lines.push("comment history: (no comments)".to_string());
                } else {
                    lines.extend(comments.into_iter().map(|comment| {
                        format!(
                            "comment {} | {} | {}\n{}",
                            comment.id, comment.author, comment.created_at, comment.body
                        )
                    }));
                }
            }

            CommandOutcome::text(EXIT_OK, lines.join("\n"))
        }
    }
}

fn render_issue_create(output: OutputFormat, view: IssueCreateView) -> CommandOutcome {
    match output {
        OutputFormat::Json { .. } => CommandOutcome::json(
            EXIT_OK,
            json!({
                "source": view.source,
                "owner": view.owner,
                "name": view.name,
                "number": view.issue.number,
                "title": view.issue.title,
                "state": view.issue.state,
                "author": view.issue.author,
                "body": view.issue.body,
                "comments": view.issue.comments,
                "html_url": view.issue.html_url,
                "created_at": view.issue.created_at,
                "updated_at": view.issue.updated_at,
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            [
                format!("Created issue {}", view.issue.number),
                format!("repository: {}/{}", view.owner, view.name),
                format!("title: {}", view.issue.title),
                format!("state: {}", view.issue.state),
                format!("author: {}", view.issue.author),
                format!("source: {}", view.source),
                format!("url: {}", view.issue.html_url),
            ]
            .join("\n"),
        ),
    }
}

fn issue_view_json(view: &IssueView) -> serde_json::Value {
    json!({
        "source": view.source,
        "owner": view.owner,
        "name": view.name,
        "number": view.issue.number,
        "title": view.issue.title,
        "state": view.issue.state,
        "author": view.issue.author,
        "body": view.issue.body,
        "comments_count": view.issue.comments,
        "html_url": view.issue.html_url,
        "created_at": view.issue.created_at,
        "updated_at": view.issue.updated_at,
        "comments_included": view.comments_included,
        "comments_page": view.comments_page,
        "comments_per_page": view.comments_per_page,
        "comments": view.comments.as_ref().map(|comments| {
            comments.iter().map(|comment| {
                json!({
                    "id": comment.id,
                    "author": comment.author,
                    "body": comment.body,
                    "created_at": comment.created_at,
                    "updated_at": comment.updated_at,
                })
            }).collect::<Vec<_>>()
        }),
    })
}

fn issue_view_selected_json(view: &IssueView, fields: &[String]) -> serde_json::Value {
    let mut selected = serde_json::Map::with_capacity(fields.len());

    for field in fields {
        let value = match field.as_str() {
            "number" => json!(view.issue.number),
            "title" => json!(view.issue.title),
            "url" => json!(view.issue.html_url),
            _ => unreachable!("unsupported issue json field"),
        };

        selected.insert(field.clone(), value);
    }

    serde_json::Value::Object(selected)
}

fn render_issue_comment(output: OutputFormat, view: IssueCommentView) -> CommandOutcome {
    match output {
        OutputFormat::Json { .. } => CommandOutcome::json(
            EXIT_OK,
            json!({
                "source": view.source,
                "owner": view.owner,
                "name": view.name,
                "number": view.number,
                "id": view.comment.id,
                "author": view.comment.author,
                "body": view.comment.body,
                "created_at": view.comment.created_at,
                "updated_at": view.comment.updated_at,
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            [
                format!("{}/{}#{}", view.owner, view.name, view.number),
                format!("comment id: {}", view.comment.id),
                format!("author: {}", view.comment.author),
                format!("created at: {}", view.comment.created_at),
                format!("updated at: {}", view.comment.updated_at),
                format!("source: {}", view.source),
                "body:".to_string(),
                view.comment.body,
            ]
            .join("\n"),
        ),
    }
}

fn resolve_issue_repo(repo: Option<&str>) -> Result<ResolvedIssueRepo, CommandError> {
    match repo {
        Some(repo) => {
            let slug = RepoSlug::parse(repo)?;
            Ok(ResolvedIssueRepo {
                owner: slug.owner,
                name: slug.name,
                source: "explicit",
            })
        }
        None => {
            let context = infer_repo_context()
                .map_err(|err| CommandError::git(format!("git context error: {err}")))?;

            Ok(ResolvedIssueRepo {
                owner: context.owner,
                name: context.name,
                source: "local",
            })
        }
    }
}

fn read_required_issue_body(
    source: IssueBodySource,
    stdin_error: &str,
    file_error: &str,
    empty_error: &str,
) -> Result<String, CommandError> {
    let body = read_issue_body(source, stdin_error, file_error)?;

    if body.trim().is_empty() {
        return Err(CommandError::usage(empty_error));
    }

    Ok(body)
}

fn read_optional_issue_body(
    source: Option<IssueBodySource>,
) -> Result<Option<String>, CommandError> {
    match source {
        Some(source) => read_required_issue_body(
            source,
            "failed to read issue body from stdin",
            "failed to read issue body file",
            "issue body cannot be empty",
        )
        .map(Some),
        None => Ok(None),
    }
}

fn read_issue_body(
    source: IssueBodySource,
    stdin_error: &str,
    file_error: &str,
) -> Result<String, CommandError> {
    let body = match source {
        IssueBodySource::Inline(body) => body,
        IssueBodySource::File(path) => {
            if path.as_os_str() == OsStr::new("-") {
                let mut input = String::new();
                io::stdin()
                    .read_to_string(&mut input)
                    .map_err(|err| CommandError::usage(format!("{stdin_error}: {err}")))?;
                input
            } else {
                fs::read_to_string(path)
                    .map_err(|err| CommandError::usage(format!("{file_error}: {err}")))?
            }
        }
    };

    Ok(body)
}

fn map_issue_list_error(error: IssueError) -> CommandError {
    map_issue_error(error, "repository not found")
}

fn map_issue_create_error(error: IssueError) -> CommandError {
    map_issue_error(error, "repository not found")
}

fn map_issue_error(error: IssueError, not_found_message: &str) -> CommandError {
    match error {
        IssueError::InvalidToken => CommandError {
            code: EXIT_AUTH,
            stdout: None,
            stderr: Some("authentication failed".to_string()),
        },
        IssueError::Transport(err) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed: {err}")),
        },
        IssueError::UnexpectedStatus(status) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!(
                "remote request returned unexpected status: {status}"
            )),
        },
        IssueError::UnexpectedStatusWithMessage(status, message) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed ({status}): {message}")),
        },
        IssueError::NotFound => CommandError::not_found(not_found_message),
    }
}
