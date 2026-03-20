use serde_json::json;

use crate::command::{CommandError, CommandOutcome, EXIT_OK, EXIT_REMOTE, OutputFormat};
use crate::config::ConfigStore;
use crate::gitee_api::{GiteeClient, RepoError, Repository};
use crate::repo_context::infer_repo_context;

pub struct RepoService {
    config: ConfigStore,
    client: GiteeClient,
}

impl RepoService {
    pub fn from_env() -> Self {
        Self {
            config: ConfigStore::from_env(),
            client: GiteeClient::from_env(),
        }
    }

    pub fn view(&self, request: RepoViewRequest) -> Result<CommandOutcome, CommandError> {
        let resolved = match request.repo {
            Some(repo) => {
                let slug = RepoSlug::parse(&repo)?;
                ResolvedRepoView {
                    owner: slug.owner,
                    name: slug.name,
                    source: "explicit",
                    current_branch: None,
                }
            }
            None => {
                let context = infer_repo_context()
                    .map_err(|err| CommandError::git(format!("git context error: {err}")))?;

                ResolvedRepoView {
                    owner: context.owner,
                    name: context.name,
                    source: "local",
                    current_branch: Some(context.current_branch),
                }
            }
        };
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);

        let repository = self
            .client
            .fetch_repository(&resolved.owner, &resolved.name, token.as_deref())
            .map_err(map_repo_error)?;

        Ok(render_repo_view(
            request.output,
            RepoView {
                source: resolved.source,
                current_branch: resolved.current_branch,
                repository,
            },
        ))
    }
}

pub struct RepoViewRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
}

struct RepoView {
    source: &'static str,
    current_branch: Option<String>,
    repository: Repository,
}

struct ResolvedRepoView {
    owner: String,
    name: String,
    source: &'static str,
    current_branch: Option<String>,
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

fn render_repo_view(output: OutputFormat, view: RepoView) -> CommandOutcome {
    match output {
        OutputFormat::Json => CommandOutcome::json(
            EXIT_OK,
            json!({
                "source": view.source,
                "owner": view.repository.owner,
                "name": view.repository.name,
                "full_name": view.repository.full_name,
                "default_branch": view.repository.default_branch,
                "current_branch": view.current_branch,
                "html_url": view.repository.html_url,
                "ssh_url": view.repository.ssh_url,
                "clone_url": view.repository.clone_url,
                "fork": view.repository.fork,
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            format!(
                "{}\ndefault branch: {}\ncurrent branch: {}\nfork: {}\nhtml url: {}\nssh url: {}\nclone url: {}\nsource: {}",
                view.repository.full_name,
                view.repository.default_branch,
                view.current_branch.as_deref().unwrap_or("(none)"),
                view.repository.fork,
                view.repository.html_url,
                view.repository.ssh_url,
                view.repository.clone_url,
                view.source,
            ),
        ),
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
