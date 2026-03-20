use std::fs;
use std::io::{self, Read};

use serde_json::json;

use crate::command::{CommandError, CommandOutcome, EXIT_OK, EXIT_REMOTE, OutputFormat};
use crate::config::ConfigStore;
use crate::gitee_api::{
    CreatePullRequestComment, GiteeClient, PullRequest, PullRequestComment, PullRequestError,
    PullRequestListFilters, RepoError,
};
use crate::repo_context::infer_repo_context;

pub struct PrService {
    config: ConfigStore,
    client: GiteeClient,
}

impl PrService {
    pub fn from_env() -> Self {
        Self {
            config: ConfigStore::from_env(),
            client: GiteeClient::from_env(),
        }
    }

    pub fn view(&self, request: PrViewRequest) -> Result<CommandOutcome, CommandError> {
        let repo = resolve_repo(request.repo.as_deref())?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);

        let pull_request =
            self.fetch_pull_request_with_fallback(&repo, request.number, token.as_deref())?;

        Ok(render_pr_view(request.output, pull_request))
    }

    pub fn comment(&self, request: PrCommentRequest) -> Result<CommandOutcome, CommandError> {
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .ok_or_else(|| CommandError {
                code: crate::command::EXIT_AUTH,
                stdout: None,
                stderr: Some("authentication required for pr comment".to_string()),
            })?
            .token;

        let repo = resolve_repo(request.repo.as_deref())?;
        let body = read_required_body(request.body)?;
        let target_repo = self.resolve_comment_target_repo(&repo, request.number, Some(&token))?;
        let comment = self
            .client
            .create_pull_request_comment(
                &target_repo.owner,
                &target_repo.name,
                request.number,
                &token,
                &CreatePullRequestComment { body: &body },
            )
            .map_err(map_pull_request_error)?;

        Ok(render_pr_comment(
            request.output,
            &target_repo,
            request.number,
            comment,
        ))
    }

    pub fn list(&self, request: PrListRequest) -> Result<CommandOutcome, CommandError> {
        let repo = resolve_repo(request.repo.as_deref())?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);

        let (repo, pull_requests) =
            self.fetch_pull_requests_with_fallback(&repo, &request.filters, token.as_deref())?;

        Ok(render_pr_list(request.output, &repo, pull_requests))
    }

    pub fn status(&self, request: PrStatusRequest) -> Result<CommandOutcome, CommandError> {
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .ok_or_else(|| CommandError {
                code: crate::command::EXIT_AUTH,
                stdout: None,
                stderr: Some("authentication required for pr status".to_string()),
            })?
            .token;

        let repo = resolve_repo(None)?;
        let current_branch = repo.current_branch.clone().ok_or_else(|| {
            CommandError::git("git context error: failed to resolve current branch")
        })?;
        let current_user = self
            .client
            .fetch_current_user(&token)
            .map_err(map_auth_error)?;

        let (repo, current_branch_prs) = self.fetch_pull_requests_with_fallback(
            &repo,
            &PullRequestListFilters {
                head: Some(current_branch.clone()),
                ..request.filters.clone()
            },
            Some(&token),
        )?;

        let (_, authored_prs) = self.fetch_pull_requests_with_fallback(
            &repo,
            &PullRequestListFilters {
                author: Some(current_user.clone()),
                ..request.filters.clone()
            },
            Some(&token),
        )?;

        let (_, assigned_prs) = self.fetch_pull_requests_with_fallback(
            &repo,
            &PullRequestListFilters {
                assignee: Some(current_user.clone()),
                ..request.filters
            },
            Some(&token),
        )?;

        Ok(render_pr_status(
            request.output,
            &repo,
            &current_user,
            &current_branch,
            current_branch_prs,
            authored_prs,
            assigned_prs,
        ))
    }

    fn fetch_pull_request_with_fallback(
        &self,
        repo: &ResolvedRepo,
        number: u64,
        token: Option<&str>,
    ) -> Result<PullRequest, CommandError> {
        match self
            .client
            .fetch_pull_request(&repo.owner, &repo.name, number, token)
        {
            Ok(pull_request) => Ok(pull_request),
            Err(PullRequestError::NotFound) => {
                let target_repo =
                    if let Some(canonical_repo) = self.find_canonical_repo(repo, token)? {
                        match self.client.fetch_pull_request(
                            &canonical_repo.owner,
                            &canonical_repo.name,
                            number,
                            token,
                        ) {
                            Ok(pull_request) => return Ok(pull_request),
                            Err(PullRequestError::NotFound) => canonical_repo,
                            Err(error) => return Err(map_pull_request_error(error)),
                        }
                    } else {
                        repo.clone()
                    };

                Err(self.classify_missing_pull_request(&target_repo, token))
            }
            Err(error) => Err(map_pull_request_error(error)),
        }
    }

    fn fetch_pull_requests_with_fallback(
        &self,
        repo: &ResolvedRepo,
        filters: &PullRequestListFilters,
        token: Option<&str>,
    ) -> Result<(ResolvedRepo, Vec<PullRequest>), CommandError> {
        match self
            .client
            .fetch_pull_requests(&repo.owner, &repo.name, filters, token)
        {
            Ok(pull_requests) => Ok((repo.clone(), pull_requests)),
            Err(RepoError::NotFound) => {
                let Some(canonical_repo) = self.find_canonical_repo(repo, token)? else {
                    return Err(map_repo_error(RepoError::NotFound));
                };

                let pull_requests = self
                    .client
                    .fetch_pull_requests(
                        &canonical_repo.owner,
                        &canonical_repo.name,
                        filters,
                        token,
                    )
                    .map_err(map_repo_error)?;

                Ok((canonical_repo, pull_requests))
            }
            Err(error) => Err(map_repo_error(error)),
        }
    }

    fn find_canonical_repo(
        &self,
        repo: &ResolvedRepo,
        token: Option<&str>,
    ) -> Result<Option<ResolvedRepo>, CommandError> {
        if !repo.allow_human_name_fallback {
            return Ok(None);
        }

        let Some(token) = token else {
            return Ok(None);
        };

        let repository = self
            .client
            .find_repository_by_human_name(&repo.owner, &repo.name, token)
            .map_err(map_repo_error)?;

        Ok(repository.map(|repository| ResolvedRepo {
            owner: repository.owner,
            name: repository.name,
            source: repo.source,
            current_branch: repo.current_branch.clone(),
            allow_human_name_fallback: false,
        }))
    }

    fn classify_missing_pull_request(
        &self,
        repo: &ResolvedRepo,
        token: Option<&str>,
    ) -> CommandError {
        match self.client.fetch_repository(&repo.owner, &repo.name, token) {
            Ok(_) => CommandError::not_found("pull request not found"),
            Err(RepoError::NotFound) => CommandError::not_found("repository not found"),
            Err(error) => map_repo_error(error),
        }
    }

    fn resolve_comment_target_repo(
        &self,
        repo: &ResolvedRepo,
        number: u64,
        token: Option<&str>,
    ) -> Result<ResolvedRepo, CommandError> {
        match self
            .client
            .fetch_pull_request(&repo.owner, &repo.name, number, token)
        {
            Ok(_) => Ok(repo.clone()),
            Err(PullRequestError::NotFound) => {
                if let Some(canonical_repo) = self.find_canonical_repo(repo, token)? {
                    match self.client.fetch_pull_request(
                        &canonical_repo.owner,
                        &canonical_repo.name,
                        number,
                        token,
                    ) {
                        Ok(_) => Ok(canonical_repo),
                        Err(PullRequestError::NotFound) => {
                            Err(self.classify_missing_pull_request(&canonical_repo, token))
                        }
                        Err(error) => Err(map_pull_request_error(error)),
                    }
                } else {
                    Err(self.classify_missing_pull_request(repo, token))
                }
            }
            Err(error) => Err(map_pull_request_error(error)),
        }
    }
}

pub struct PrViewRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub number: u64,
}

pub struct PrCommentRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub number: u64,
    pub body: PrTextSource,
}

pub struct PrListRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
    pub filters: PullRequestListFilters,
}

pub struct PrStatusRequest {
    pub output: OutputFormat,
    pub filters: PullRequestListFilters,
}

pub enum PrTextSource {
    Inline(String),
    File(String),
}

#[derive(Clone)]
struct ResolvedRepo {
    owner: String,
    name: String,
    source: &'static str,
    current_branch: Option<String>,
    allow_human_name_fallback: bool,
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

fn resolve_repo(repo: Option<&str>) -> Result<ResolvedRepo, CommandError> {
    match repo {
        Some(repo) => {
            let slug = RepoSlug::parse(repo)?;
            Ok(ResolvedRepo {
                owner: slug.owner,
                name: slug.name,
                source: "explicit",
                current_branch: None,
                allow_human_name_fallback: false,
            })
        }
        None => {
            let context = infer_repo_context()
                .map_err(|err| CommandError::git(format!("git context error: {err}")))?;

            Ok(ResolvedRepo {
                owner: context.owner,
                name: context.name,
                source: "local",
                current_branch: Some(context.current_branch),
                allow_human_name_fallback: true,
            })
        }
    }
}

fn render_pr_view(output: OutputFormat, pull_request: PullRequest) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            EXIT_OK,
            json!({
                "number": pull_request.number,
                "state": pull_request.state,
                "title": pull_request.title,
                "body": pull_request.body,
                "author": pull_request.author,
                "repository": pull_request.repository,
                "head_ref": pull_request.head.r#ref,
                "head_sha": pull_request.head.sha,
                "head_repository": pull_request.head.repository,
                "base_ref": pull_request.base.r#ref,
                "base_sha": pull_request.base.sha,
                "base_repository": pull_request.base.repository,
                "draft": pull_request.draft,
                "mergeable": pull_request.mergeable,
                "html_url": pull_request.html_url,
                "created_at": pull_request.created_at,
                "updated_at": pull_request.updated_at,
                "merged_at": pull_request.merged_at,
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            format!(
                "#{} {}\nstate: {}\nauthor: {}\nrepository: {}\nhead: {}:{}\nbase: {}:{}\ndraft: {}\nmergeable: {}\nurl: {}",
                pull_request.number,
                pull_request.title,
                pull_request.state,
                pull_request.author,
                pull_request.repository,
                pull_request.head.repository,
                pull_request.head.r#ref,
                pull_request.base.repository,
                pull_request.base.r#ref,
                pull_request.draft,
                render_optional_bool(pull_request.mergeable),
                pull_request.html_url,
            ),
        ),
    }
}

fn render_pr_comment(
    output: OutputFormat,
    repo: &ResolvedRepo,
    number: u64,
    comment: PullRequestComment,
) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            EXIT_OK,
            json!({
                "id": comment.id,
                "body": comment.body,
                "author": comment.author,
                "repository": format!("{}/{}", repo.owner, repo.name),
                "pull_request": number,
                "html_url": comment.html_url,
                "created_at": comment.created_at,
                "updated_at": comment.updated_at,
                "comment_type": comment.comment_type,
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            format!(
                "Commented on pull request #{number}\ncomment id: {}\nrepository: {}/{}\nauthor: {}\nurl: {}",
                comment.id, repo.owner, repo.name, comment.author, comment.html_url,
            ),
        ),
    }
}

fn render_pr_list(
    output: OutputFormat,
    repo: &ResolvedRepo,
    pull_requests: Vec<PullRequest>,
) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            EXIT_OK,
            json!({
                "repository": format!("{}/{}", repo.owner, repo.name),
                "source": repo.source,
                "count": pull_requests.len(),
                "pull_requests": pull_requests.iter().map(pr_summary_json).collect::<Vec<_>>(),
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(EXIT_OK, render_pr_list_text(&pull_requests)),
    }
}

fn render_pr_status(
    output: OutputFormat,
    repo: &ResolvedRepo,
    current_user: &str,
    current_branch: &str,
    current_branch_prs: Vec<PullRequest>,
    authored_prs: Vec<PullRequest>,
    assigned_prs: Vec<PullRequest>,
) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            EXIT_OK,
            json!({
                "repository": format!("{}/{}", repo.owner, repo.name),
                "source": repo.source,
                "current_user": current_user,
                "current_branch": current_branch,
                "current_branch_prs": current_branch_prs.iter().map(pr_summary_json).collect::<Vec<_>>(),
                "authored_prs": authored_prs.iter().map(pr_summary_json).collect::<Vec<_>>(),
                "assigned_prs": assigned_prs.iter().map(pr_summary_json).collect::<Vec<_>>(),
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            format!(
                "Current user: {current_user}\nCurrent branch: {current_branch}\n\nCurrent branch\n{}\n\nAuthored by you\n{}\n\nAssigned to you\n{}",
                render_pr_list_text(&current_branch_prs),
                render_pr_list_text(&authored_prs),
                render_pr_list_text(&assigned_prs),
            ),
        ),
    }
}

fn render_pr_list_text(pull_requests: &[PullRequest]) -> String {
    if pull_requests.is_empty() {
        return "No pull requests found".to_string();
    }

    pull_requests
        .iter()
        .map(|pull_request| {
            format!(
                "#{} {} {} ({})",
                pull_request.number, pull_request.state, pull_request.title, pull_request.author,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn pr_summary_json(pull_request: &PullRequest) -> serde_json::Value {
    json!({
        "number": pull_request.number,
        "state": pull_request.state,
        "title": pull_request.title,
        "author": pull_request.author,
        "repository": pull_request.repository,
        "head_ref": pull_request.head.r#ref,
        "head_sha": pull_request.head.sha,
        "head_repository": pull_request.head.repository,
        "base_ref": pull_request.base.r#ref,
        "base_sha": pull_request.base.sha,
        "base_repository": pull_request.base.repository,
        "draft": pull_request.draft,
        "mergeable": pull_request.mergeable,
        "html_url": pull_request.html_url,
        "created_at": pull_request.created_at,
        "updated_at": pull_request.updated_at,
        "merged_at": pull_request.merged_at,
    })
}

fn render_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn read_required_body(body: PrTextSource) -> Result<String, CommandError> {
    let body = match body {
        PrTextSource::Inline(value) => value,
        PrTextSource::File(path) => read_body_from_file(&path)?,
    };

    if body.trim().is_empty() {
        return Err(CommandError::usage("comment body cannot be empty"));
    }

    Ok(body)
}

fn read_body_from_file(path: &str) -> Result<String, CommandError> {
    if path == "-" {
        let mut body = String::new();
        io::stdin()
            .read_to_string(&mut body)
            .map_err(|err| CommandError::usage(format!("failed to read stdin: {err}")))?;
        return Ok(body);
    }

    fs::read_to_string(path)
        .map_err(|err| CommandError::usage(format!("failed to read body file `{path}`: {err}")))
}

fn map_pull_request_error(error: PullRequestError) -> CommandError {
    match error {
        PullRequestError::InvalidToken => CommandError {
            code: crate::command::EXIT_AUTH,
            stdout: None,
            stderr: Some("authentication failed".to_string()),
        },
        PullRequestError::Transport(err) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed: {err}")),
        },
        PullRequestError::UnexpectedStatus(status) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!(
                "remote request returned unexpected status: {status}"
            )),
        },
        PullRequestError::NotFound => CommandError::not_found("pull request not found"),
    }
}

fn map_repo_error(error: RepoError) -> CommandError {
    match error {
        RepoError::InvalidToken => CommandError {
            code: crate::command::EXIT_AUTH,
            stdout: None,
            stderr: Some("authentication failed".to_string()),
        },
        RepoError::Transport(err) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed: {err}")),
        },
        RepoError::UnexpectedStatus(status) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!(
                "remote request returned unexpected status: {status}"
            )),
        },
        RepoError::NotFound => CommandError::not_found("repository not found"),
    }
}

fn map_auth_error(error: crate::gitee_api::AuthError) -> CommandError {
    match error {
        crate::gitee_api::AuthError::InvalidToken => CommandError {
            code: crate::command::EXIT_AUTH,
            stdout: None,
            stderr: Some("authentication failed".to_string()),
        },
        crate::gitee_api::AuthError::Transport(err) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!("remote request failed: {err}")),
        },
        crate::gitee_api::AuthError::UnexpectedStatus(status) => CommandError {
            code: EXIT_REMOTE,
            stdout: None,
            stderr: Some(format!(
                "remote request returned unexpected status: {status}"
            )),
        },
    }
}
