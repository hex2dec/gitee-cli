use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde_json::json;

use crate::command::{CommandError, CommandOutcome, EXIT_GIT, EXIT_OK, EXIT_REMOTE, OutputFormat};
use crate::config::{CloneProtocol, ConfigStore};
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
                    allow_human_name_fallback: false,
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
                    allow_human_name_fallback: true,
                }
            }
        };
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);

        let repository =
            match self
                .client
                .fetch_repository(&resolved.owner, &resolved.name, token.as_deref())
            {
                Ok(repository) => repository,
                Err(RepoError::NotFound) if resolved.allow_human_name_fallback => {
                    let Some(token) = token.as_deref() else {
                        return Err(map_repo_error(RepoError::NotFound));
                    };

                    self.client
                        .find_repository_by_human_name(&resolved.owner, &resolved.name, token)
                        .map_err(map_repo_error)?
                        .ok_or_else(|| map_repo_error(RepoError::NotFound))?
                }
                Err(error) => return Err(map_repo_error(error)),
            };

        Ok(render_repo_view(
            request.output,
            RepoView {
                source: resolved.source,
                current_branch: resolved.current_branch,
                repository,
            },
        ))
    }

    pub fn clone(&self, request: RepoCloneRequest) -> Result<CommandOutcome, CommandError> {
        let slug = RepoSlug::parse_positional(&request.repo)?;
        let token = self
            .config
            .load_runtime_token()
            .map_err(CommandError::config)?
            .map(|resolved| resolved.token);
        let repository = self
            .client
            .fetch_repository(&slug.owner, &slug.name, token.as_deref())
            .map_err(map_repo_error)?;
        let transport = self.resolve_clone_transport(request.transport)?;
        let clone_url = transport.select_url(&repository).to_string();
        let destination = resolve_clone_destination(request.destination.as_deref(), &repository);

        ensure_clone_destination_is_available(&destination)?;
        run_git_clone(&clone_url, &destination)?;

        Ok(render_repo_clone(
            request.output,
            RepoCloneView {
                repository,
                clone_url,
                transport,
                destination: destination.canonicalize().unwrap_or(destination),
            },
        ))
    }

    fn resolve_clone_transport(
        &self,
        requested: Option<CloneTransport>,
    ) -> Result<CloneTransport, CommandError> {
        if let Some(transport) = requested {
            return Ok(transport);
        }

        if let Some(protocol) = self
            .config
            .load_clone_protocol()
            .map_err(CommandError::config)?
        {
            return Ok(protocol.into());
        }

        let transport = prompt_for_clone_transport()?;
        self.config
            .save_clone_protocol(transport.into())
            .map_err(CommandError::config)?;

        Ok(transport)
    }
}

pub struct RepoViewRequest {
    pub output: OutputFormat,
    pub repo: Option<String>,
}

pub struct RepoCloneRequest {
    pub output: OutputFormat,
    pub repo: String,
    pub destination: Option<String>,
    pub transport: Option<CloneTransport>,
}

#[derive(Clone, Copy)]
pub enum CloneTransport {
    Https,
    Ssh,
}

struct RepoView {
    source: &'static str,
    current_branch: Option<String>,
    repository: Repository,
}

struct RepoCloneView {
    repository: Repository,
    clone_url: String,
    transport: CloneTransport,
    destination: PathBuf,
}

struct ResolvedRepoView {
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

    fn parse_positional(value: &str) -> Result<Self, CommandError> {
        let Some((owner, name)) = value.split_once('/') else {
            return Err(CommandError::usage(
                "invalid repository slug: expected owner/repo",
            ));
        };

        if owner.is_empty() || name.is_empty() || name.contains('/') {
            return Err(CommandError::usage(
                "invalid repository slug: expected owner/repo",
            ));
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }
}

impl CloneTransport {
    fn as_str(self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Ssh => "ssh",
        }
    }

    fn parse_choice(input: &str) -> Result<Self, CommandError> {
        match input.trim().to_ascii_lowercase().as_str() {
            "https" | "2" => Ok(Self::Https),
            "ssh" | "1" => Ok(Self::Ssh),
            _ => Err(CommandError::usage(
                "clone protocol must be selected as ssh or https",
            )),
        }
    }

    fn select_url(self, repository: &Repository) -> &str {
        match self {
            Self::Https => &repository.clone_url,
            Self::Ssh => &repository.ssh_url,
        }
    }
}

impl From<CloneProtocol> for CloneTransport {
    fn from(value: CloneProtocol) -> Self {
        match value {
            CloneProtocol::Https => Self::Https,
            CloneProtocol::Ssh => Self::Ssh,
        }
    }
}

impl From<CloneTransport> for CloneProtocol {
    fn from(value: CloneTransport) -> Self {
        match value {
            CloneTransport::Https => Self::Https,
            CloneTransport::Ssh => Self::Ssh,
        }
    }
}

fn render_repo_view(output: OutputFormat, view: RepoView) -> CommandOutcome {
    match output {
        OutputFormat::Json { fields } => CommandOutcome::json(
            EXIT_OK,
            match fields {
                Some(fields) => repo_view_selected_json(&view, &fields),
                None => repo_view_json(&view),
            },
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

fn render_repo_clone(output: OutputFormat, view: RepoCloneView) -> CommandOutcome {
    match output {
        OutputFormat::Json { .. } => CommandOutcome::json(
            EXIT_OK,
            json!({
                "owner": view.repository.owner,
                "name": view.repository.name,
                "full_name": view.repository.full_name,
                "transport": view.transport.as_str(),
                "clone_url": view.clone_url,
                "destination": view.destination.display().to_string(),
            }),
        ),
        OutputFormat::Text => CommandOutcome::text(
            EXIT_OK,
            format!(
                "Cloned {} to {} via {}",
                view.repository.full_name,
                view.destination.display(),
                view.transport.as_str(),
            ),
        ),
    }
}

fn repo_view_json(view: &RepoView) -> serde_json::Value {
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
    })
}

fn repo_view_selected_json(view: &RepoView, fields: &[String]) -> serde_json::Value {
    let mut selected = serde_json::Map::with_capacity(fields.len());

    for field in fields {
        let value = match field.as_str() {
            "name" => json!(view.repository.name),
            "nameWithOwner" => json!(view.repository.full_name),
            "url" => json!(view.repository.html_url),
            _ => unreachable!("unsupported repository json field"),
        };

        selected.insert(field.clone(), value);
    }

    serde_json::Value::Object(selected)
}

fn resolve_clone_destination(destination: Option<&str>, repository: &Repository) -> PathBuf {
    destination
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&repository.name))
}

fn ensure_clone_destination_is_available(destination: &Path) -> Result<(), CommandError> {
    if !destination.exists() {
        return Ok(());
    }

    if destination.is_dir() {
        let mut entries = fs::read_dir(destination).map_err(|err| CommandError {
            code: EXIT_GIT,
            stdout: None,
            stderr: Some(format!("failed to inspect clone destination: {err}")),
        })?;

        if entries.next().is_none() {
            return Ok(());
        }
    }

    Err(CommandError::git(format!(
        "clone destination already exists: {}",
        destination.display()
    )))
}

fn run_git_clone(clone_url: &str, destination: &Path) -> Result<(), CommandError> {
    let mut child = Command::new("git")
        .arg("clone")
        .arg("--progress")
        .arg(clone_url)
        .arg(destination)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| CommandError::git(format!("failed to run git: {err}")))?;

    let Some(stderr) = child.stderr.take() else {
        return Err(CommandError::git("failed to capture git stderr"));
    };

    let stderr_thread = thread::spawn(move || -> Result<Vec<u8>, io::Error> {
        let mut reader = stderr;
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 8192];
        let mut writer = io::stderr().lock();

        loop {
            let bytes_read = reader.read(&mut chunk)?;
            if bytes_read == 0 {
                break;
            }

            writer.write_all(&chunk[..bytes_read])?;
            writer.flush()?;
            buffer.extend_from_slice(&chunk[..bytes_read]);
        }

        Ok(buffer)
    });

    let status = child
        .wait()
        .map_err(|err| CommandError::git(format!("failed to wait for git: {err}")))?;
    let stderr_bytes = stderr_thread
        .join()
        .map_err(|_| CommandError::git("failed to read git stderr"))?
        .map_err(|err| CommandError::git(format!("failed to read git stderr: {err}")))?;

    if status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
    let message = if stderr.is_empty() {
        "git clone failed".to_string()
    } else {
        format!("git clone failed: {stderr}")
    };

    Err(CommandError::git(message))
}

fn prompt_for_clone_transport() -> Result<CloneTransport, CommandError> {
    let mut stderr = io::stderr();
    writeln!(stderr, "No saved clone protocol preference.")
        .map_err(|err| CommandError::usage(format!("failed to write prompt: {err}")))?;
    write!(stderr, "Choose clone protocol [ssh/https]: ")
        .map_err(|err| CommandError::usage(format!("failed to write prompt: {err}")))?;
    stderr
        .flush()
        .map_err(|err| CommandError::usage(format!("failed to write prompt: {err}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CommandError::usage(format!("failed to read clone protocol: {err}")))?;

    CloneTransport::parse_choice(&input)
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
