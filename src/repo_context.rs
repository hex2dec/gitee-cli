use std::process::Command;

pub struct InferredRepoContext {
    pub owner: String,
    pub name: String,
    pub current_branch: String,
}

pub enum RepoContextError {
    NotGitRepository,
    DetachedHead,
    MissingOriginRemote,
    UnsupportedRemote,
    GitCommandFailed(String),
}

impl std::fmt::Display for RepoContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGitRepository => write!(f, "not inside a git repository"),
            Self::DetachedHead => write!(f, "HEAD is detached"),
            Self::MissingOriginRemote => write!(f, "missing origin remote"),
            Self::UnsupportedRemote => {
                write!(
                    f,
                    "origin remote is not a supported gitee.com repository URL"
                )
            }
            Self::GitCommandFailed(message) => write!(f, "{message}"),
        }
    }
}

pub fn infer_repo_context() -> Result<InferredRepoContext, RepoContextError> {
    ensure_git_repository()?;
    let current_branch = current_branch()?;
    let remote_url = origin_remote_url()?;
    let (owner, name) = parse_gitee_remote(&remote_url)?;

    Ok(InferredRepoContext {
        owner,
        name,
        current_branch,
    })
}

fn ensure_git_repository() -> Result<(), RepoContextError> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|err| RepoContextError::GitCommandFailed(format!("failed to run git: {err}")))?;

    if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true" {
        return Ok(());
    }

    Err(RepoContextError::NotGitRepository)
}

fn current_branch() -> Result<String, RepoContextError> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .map_err(|err| RepoContextError::GitCommandFailed(format!("failed to run git: {err}")))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    if output.status.code() == Some(1) {
        return Err(RepoContextError::DetachedHead);
    }

    Err(RepoContextError::GitCommandFailed(
        "failed to resolve current branch".to_string(),
    ))
}

fn origin_remote_url() -> Result<String, RepoContextError> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|err| RepoContextError::GitCommandFailed(format!("failed to run git: {err}")))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    Err(RepoContextError::MissingOriginRemote)
}

fn parse_gitee_remote(remote_url: &str) -> Result<(String, String), RepoContextError> {
    let slug = if let Some(value) = remote_url.strip_prefix("git@gitee.com:") {
        value
    } else if let Some(value) = remote_url.strip_prefix("ssh://git@gitee.com/") {
        value
    } else if let Some(value) = remote_url.strip_prefix("https://gitee.com/") {
        value
    } else if let Some(value) = remote_url.strip_prefix("http://gitee.com/") {
        value
    } else {
        return Err(RepoContextError::UnsupportedRemote);
    };

    let slug = slug.trim_end_matches(".git");
    let mut parts = slug.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();

    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return Err(RepoContextError::UnsupportedRemote);
    }

    Ok((owner.to_string(), name.to_string()))
}
