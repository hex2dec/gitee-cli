use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug)]
pub struct InferredRepoContext {
    pub owner: String,
    pub name: String,
    pub current_branch: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RepoContextError {
    NotGitRepository,
    DetachedHead,
    MissingOriginRemote,
    UnsupportedRemote,
    CurrentBranchNotPushedToOrigin,
    CurrentBranchTracksNonOriginRemote(String),
    CurrentBranchTracksUnexpectedRef { actual: String, expected: String },
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
            Self::CurrentBranchNotPushedToOrigin => {
                write!(f, "current branch is not pushed to origin")
            }
            Self::CurrentBranchTracksNonOriginRemote(remote) => {
                write!(
                    f,
                    "current branch tracks remote `{remote}`, expected origin"
                )
            }
            Self::CurrentBranchTracksUnexpectedRef { actual, expected } => {
                write!(f, "current branch tracks `{actual}`, expected {expected}")
            }
            Self::GitCommandFailed(message) => write!(f, "{message}"),
        }
    }
}

pub fn infer_repo_context() -> Result<InferredRepoContext, RepoContextError> {
    GitContextResolver::new().infer_repo_context()
}

pub fn infer_repo_context_with_pushed_branch() -> Result<InferredRepoContext, RepoContextError> {
    GitContextResolver::new().infer_repo_context_with_pushed_branch()
}

struct GitContextResolver<'a> {
    current_dir: Option<&'a Path>,
}

impl GitContextResolver<'_> {
    fn new() -> Self {
        Self { current_dir: None }
    }
}

impl<'a> GitContextResolver<'a> {
    #[cfg(test)]
    fn in_dir(current_dir: &'a Path) -> Self {
        Self {
            current_dir: Some(current_dir),
        }
    }

    fn infer_repo_context(&self) -> Result<InferredRepoContext, RepoContextError> {
        self.ensure_git_repository()?;
        let current_branch = self.current_branch()?;
        let remote_url = self.origin_remote_url()?;
        let (owner, name) = parse_gitee_remote(&remote_url)?;

        Ok(InferredRepoContext {
            owner,
            name,
            current_branch,
        })
    }

    fn infer_repo_context_with_pushed_branch(
        &self,
    ) -> Result<InferredRepoContext, RepoContextError> {
        let context = self.infer_repo_context()?;
        self.ensure_current_branch_is_pushed_to_origin(&context.current_branch)?;
        Ok(context)
    }

    fn ensure_git_repository(&self) -> Result<(), RepoContextError> {
        let output = self.run_git(["rev-parse", "--is-inside-work-tree"])?;

        if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true" {
            return Ok(());
        }

        Err(RepoContextError::NotGitRepository)
    }

    fn current_branch(&self) -> Result<String, RepoContextError> {
        let output = self.run_git(["symbolic-ref", "--quiet", "--short", "HEAD"])?;

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

    fn origin_remote_url(&self) -> Result<String, RepoContextError> {
        let output = self.run_git(["remote", "get-url", "origin"])?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        Err(RepoContextError::MissingOriginRemote)
    }

    fn ensure_current_branch_is_pushed_to_origin(
        &self,
        branch: &str,
    ) -> Result<(), RepoContextError> {
        let Some(remote) = self.git_config(&format!("branch.{branch}.remote"))? else {
            return Err(RepoContextError::CurrentBranchNotPushedToOrigin);
        };

        if remote != "origin" {
            return Err(RepoContextError::CurrentBranchTracksNonOriginRemote(remote));
        }

        let expected_merge = format!("refs/heads/{branch}");
        let Some(merge_ref) = self.git_config(&format!("branch.{branch}.merge"))? else {
            return Err(RepoContextError::CurrentBranchNotPushedToOrigin);
        };

        if merge_ref != expected_merge {
            return Err(RepoContextError::CurrentBranchTracksUnexpectedRef {
                actual: merge_ref,
                expected: expected_merge,
            });
        }

        if !self.git_ref_exists(&format!("refs/remotes/origin/{branch}"))? {
            return Err(RepoContextError::CurrentBranchNotPushedToOrigin);
        }

        Ok(())
    }

    fn git_config(&self, key: &str) -> Result<Option<String>, RepoContextError> {
        let output = self.run_git(["config", "--get", key])?;

        if output.status.success() {
            return Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ));
        }

        if output.status.code() == Some(1) {
            return Ok(None);
        }

        Err(RepoContextError::GitCommandFailed(format!(
            "failed to read git config `{key}`"
        )))
    }

    fn git_ref_exists(&self, reference: &str) -> Result<bool, RepoContextError> {
        let output = self.run_git(["show-ref", "--verify", "--quiet", reference])?;

        if output.status.success() {
            return Ok(true);
        }

        if output.status.code() == Some(1) {
            return Ok(false);
        }

        Err(RepoContextError::GitCommandFailed(format!(
            "failed to inspect git reference `{reference}`"
        )))
    }

    fn run_git<const N: usize>(&self, args: [&str; N]) -> Result<Output, RepoContextError> {
        let mut command = Command::new("git");
        command.args(args);

        if let Some(current_dir) = self.current_dir {
            command.current_dir(current_dir);
        }

        command
            .output()
            .map_err(|err| RepoContextError::GitCommandFailed(format!("failed to run git: {err}")))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as ProcessCommand;
    use tempfile::TempDir;

    #[test]
    fn infers_context_from_https_origin() {
        let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/https");

        let context = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context()
            .unwrap();

        assert_eq!(context.owner, "octo");
        assert_eq!(context.name, "demo");
        assert_eq!(context.current_branch, "feature/https");
    }

    #[test]
    fn infers_context_from_ssh_origin() {
        let repo_dir = git_repo_with_remote("git@gitee.com:octo/demo.git", "feature/ssh");

        let context = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context()
            .unwrap();

        assert_eq!(context.owner, "octo");
        assert_eq!(context.name, "demo");
        assert_eq!(context.current_branch, "feature/ssh");
    }

    #[test]
    fn reports_missing_origin_remote() {
        let repo_dir = git_repo_without_remote("feature/no-origin");

        let error = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context()
            .unwrap_err();

        assert_eq!(error, RepoContextError::MissingOriginRemote);
        assert_eq!(error.to_string(), "missing origin remote");
    }

    #[test]
    fn reports_detached_head() {
        let repo_dir = git_repo_with_detached_head("https://gitee.com/octo/demo.git");

        let error = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context()
            .unwrap_err();

        assert_eq!(error, RepoContextError::DetachedHead);
        assert_eq!(error.to_string(), "HEAD is detached");
    }

    #[test]
    fn validates_branch_pushed_to_origin() {
        let repo_dir =
            git_repo_with_remote_and_commit("https://gitee.com/octo/demo.git", "feature/pushed");
        set_branch_upstream(
            repo_dir.path(),
            "feature/pushed",
            "origin",
            "feature/pushed",
        );

        let context = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context_with_pushed_branch()
            .unwrap();

        assert_eq!(context.owner, "octo");
        assert_eq!(context.name, "demo");
        assert_eq!(context.current_branch, "feature/pushed");
    }

    #[test]
    fn reports_branch_not_pushed_to_origin() {
        let repo_dir =
            git_repo_with_remote_and_commit("https://gitee.com/octo/demo.git", "feature/unpushed");

        let error = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context_with_pushed_branch()
            .unwrap_err();

        assert_eq!(error, RepoContextError::CurrentBranchNotPushedToOrigin);
        assert_eq!(error.to_string(), "current branch is not pushed to origin");
    }

    #[test]
    fn reports_branch_tracking_non_origin_remote() {
        let repo_dir =
            git_repo_with_remote_and_commit("https://gitee.com/octo/demo.git", "feature/forked");
        run_git(
            repo_dir.path(),
            &[
                "remote",
                "add",
                "fork",
                "https://gitee.com/octo-fork/demo.git",
            ],
        );
        set_branch_upstream(repo_dir.path(), "feature/forked", "fork", "feature/forked");

        let error = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context_with_pushed_branch()
            .unwrap_err();

        assert_eq!(
            error,
            RepoContextError::CurrentBranchTracksNonOriginRemote("fork".to_string())
        );
        assert_eq!(
            error.to_string(),
            "current branch tracks remote `fork`, expected origin"
        );
    }

    #[test]
    fn reports_branch_tracking_unexpected_origin_ref() {
        let repo_dir =
            git_repo_with_remote_and_commit("https://gitee.com/octo/demo.git", "feature/local");
        set_branch_upstream(repo_dir.path(), "feature/local", "origin", "feature/remote");

        let error = GitContextResolver::in_dir(repo_dir.path())
            .infer_repo_context_with_pushed_branch()
            .unwrap_err();

        assert_eq!(
            error,
            RepoContextError::CurrentBranchTracksUnexpectedRef {
                actual: "refs/heads/feature/remote".to_string(),
                expected: "refs/heads/feature/local".to_string(),
            }
        );
        assert_eq!(
            error.to_string(),
            "current branch tracks `refs/heads/feature/remote`, expected refs/heads/feature/local"
        );
    }

    fn git_repo_without_remote(branch: &str) -> TempDir {
        let repo_dir = TempDir::new().unwrap();

        run_git(repo_dir.path(), &["init"]);
        run_git(repo_dir.path(), &["checkout", "-b", branch]);

        repo_dir
    }

    fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
        let repo_dir = git_repo_without_remote(branch);
        run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);

        repo_dir
    }

    fn git_repo_with_remote_and_commit(remote_url: &str, branch: &str) -> TempDir {
        let repo_dir = TempDir::new().unwrap();

        run_git(repo_dir.path(), &["init"]);
        std::fs::write(repo_dir.path().join("README.md"), "hello\n").unwrap();
        run_git(repo_dir.path(), &["add", "README.md"]);
        run_git(
            repo_dir.path(),
            &[
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "init",
            ],
        );
        run_git(repo_dir.path(), &["checkout", "-b", branch]);
        run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);

        repo_dir
    }

    fn git_repo_with_detached_head(remote_url: &str) -> TempDir {
        let repo_dir = git_repo_with_remote_and_commit(remote_url, "feature/detached");
        run_git(repo_dir.path(), &["checkout", "--detach"]);

        repo_dir
    }

    fn set_branch_upstream(repo_dir: &Path, branch: &str, remote: &str, remote_branch: &str) {
        run_git(
            repo_dir,
            &["config", &format!("branch.{branch}.remote"), remote],
        );
        run_git(
            repo_dir,
            &[
                "config",
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{remote_branch}"),
            ],
        );
        run_git(
            repo_dir,
            &[
                "update-ref",
                &format!("refs/remotes/{remote}/{remote_branch}"),
                "HEAD",
            ],
        );
    }

    fn run_git(repo_dir: &Path, args: &[&str]) {
        let output = ProcessCommand::new("git")
            .args(args)
            .current_dir(repo_dir)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "git command failed: git {}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
