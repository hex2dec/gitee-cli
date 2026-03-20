use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_checkout_fetches_and_checks_out_a_pull_request_branch_in_json_output() {
    let server = MockServer::start();
    let fixture = checkout_fixture();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(pull_request_payload(
            42,
            "feature/pr-checkout",
            &fixture.head_sha,
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(&fixture.working_repo)
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "checkout", "42", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["pull_request"], 42);
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["branch"], "feature/pr-checkout");
    assert_eq!(body["current_branch"], "feature/pr-checkout");
    assert_eq!(body["created"], true);
    assert_eq!(body["head_sha"], fixture.head_sha);

    assert_eq!(current_branch(&fixture.working_repo), "feature/pr-checkout");
    assert_eq!(
        std::fs::read_to_string(fixture.working_repo.join("README.md")).unwrap(),
        "feature branch\n"
    );

    pr_mock.assert_hits(1);
}

#[test]
fn pr_checkout_reuses_an_existing_local_branch_on_repeat_invocation() {
    let server = MockServer::start();
    let fixture = checkout_fixture();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(pull_request_payload(
            42,
            "feature/pr-checkout",
            &fixture.head_sha,
        ));
    });

    let first_output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(&fixture.working_repo)
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "checkout", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(first_output.status.code(), Some(0));

    let second_output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(&fixture.working_repo)
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "checkout", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(second_output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&second_output.stdout).trim(),
        "Checked out feature/pr-checkout for pull request #42 (existing)"
    );
    assert_eq!(current_branch(&fixture.working_repo), "feature/pr-checkout");

    pr_mock.assert_hits(2);
}

#[test]
fn pr_checkout_reports_a_missing_pull_request() {
    let server = MockServer::start();
    let fixture = checkout_fixture();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/404");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(&fixture.working_repo)
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "checkout", "404", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "pull request not found"
    );

    pr_mock.assert_hits(1);
    repo_mock.assert_hits(1);
}

#[test]
fn pr_checkout_requires_a_local_git_repository() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pr", "checkout", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "git context error: not inside a git repository"
    );
}

#[test]
fn pr_checkout_surfaces_git_checkout_conflicts() {
    let server = MockServer::start();
    let fixture = checkout_fixture();
    write_file(&fixture.working_repo, "README.md", "local change\n");

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(pull_request_payload(
            42,
            "feature/pr-checkout",
            &fixture.head_sha,
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(&fixture.working_repo)
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "checkout", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("git checkout failed:"));
    assert!(stderr.contains("README.md"));

    pr_mock.assert_hits(1);
}

struct CheckoutFixture {
    _remote_repo: TempDir,
    _working_root: TempDir,
    working_repo: PathBuf,
    head_sha: String,
}

fn checkout_fixture() -> CheckoutFixture {
    let remote_repo = TempDir::new().unwrap();
    let source_repo = TempDir::new().unwrap();
    let working_root = TempDir::new().unwrap();
    let working_repo = working_root.path().join("demo");

    run_git(remote_repo.path(), &["init", "--bare"]);
    run_git(source_repo.path(), &["init"]);
    write_file(source_repo.path(), "README.md", "main branch\n");
    run_git(source_repo.path(), &["add", "README.md"]);
    commit_all(source_repo.path(), "seed main");
    run_git(source_repo.path(), &["branch", "-M", "main"]);
    run_git(
        source_repo.path(),
        &[
            "remote",
            "add",
            "origin",
            remote_repo.path().to_str().unwrap(),
        ],
    );
    run_git(source_repo.path(), &["push", "-u", "origin", "main"]);
    run_git(
        source_repo.path(),
        &["checkout", "-b", "feature/pr-checkout"],
    );
    write_file(source_repo.path(), "README.md", "feature branch\n");
    run_git(source_repo.path(), &["add", "README.md"]);
    commit_all(source_repo.path(), "feature commit");
    let head_sha = git_stdout(source_repo.path(), &["rev-parse", "HEAD"]);
    run_git(
        source_repo.path(),
        &["push", "-u", "origin", "feature/pr-checkout"],
    );
    run_git(
        remote_repo.path(),
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    );
    run_git(
        working_root.path(),
        &[
            "clone",
            remote_repo.path().to_str().unwrap(),
            working_repo.file_name().unwrap().to_str().unwrap(),
        ],
    );

    CheckoutFixture {
        _remote_repo: remote_repo,
        _working_root: working_root,
        working_repo,
        head_sha,
    }
}

fn pull_request_payload(number: u64, head_ref: &str, head_sha: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": "Checkout target",
        "body": null,
        "html_url": format!("https://gitee.com/octo/demo/pulls/{number}"),
        "draft": false,
        "mergeable": true,
        "created_at": "2026-03-20T09:00:00+08:00",
        "updated_at": "2026-03-20T10:00:00+08:00",
        "merged_at": null,
        "user": {
            "login": "octocat"
        },
        "head": {
            "ref": head_ref,
            "sha": head_sha,
            "repo": {
                "full_name": "octo/demo"
            }
        },
        "base": {
            "ref": "main",
            "sha": "def456",
            "repo": {
                "full_name": "octo/demo"
            }
        }
    })
}

fn current_branch(repo_dir: &Path) -> String {
    git_stdout(repo_dir, &["symbolic-ref", "--quiet", "--short", "HEAD"])
}

fn commit_all(repo_dir: &Path, message: &str) {
    run_git(
        repo_dir,
        &[
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            message,
        ],
    );
}

fn write_file(repo_dir: &Path, name: &str, contents: &str) {
    std::fs::write(repo_dir.join(name), contents).unwrap();
}

fn git_stdout(repo_dir: &Path, args: &[&str]) -> String {
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

    String::from_utf8_lossy(&output.stdout).trim().to_string()
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
