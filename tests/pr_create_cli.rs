use assert_cmd::Command;
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_create_uses_explicit_head_and_inferred_base_in_json_output() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo")
            .query_param("access_token", "secret-token");
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

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls")
            .header("content-type", "application/json")
            .body_contains("\"access_token\":\"secret-token\"")
            .body_contains("\"title\":\"Add PR create\"")
            .body_contains("\"head\":\"feature/pr-create\"")
            .body_contains("\"base\":\"main\"")
            .body_contains("\"body\":\"Creates the pull request\"");
        then.status(201).json_body(pull_request_payload(
            42,
            "Add PR create",
            Some("Creates the pull request"),
            "octocat",
            "feature/pr-create",
            "main",
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/pr-create",
            "--title",
            "Add PR create",
            "--body",
            "Creates the pull request",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "Add PR create");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["head_ref"], "feature/pr-create");
    assert_eq!(body["base_ref"], "main");
    assert_eq!(body["body"], "Creates the pull request");

    repo_mock.assert_hits(1);
    create_mock.assert_hits(1);
}

#[test]
fn pr_create_infers_local_head_and_renders_text_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_gitee_origin("feature/local-head");
    set_branch_upstream(
        repo_dir.path(),
        "feature/local-head",
        "origin",
        "feature/local-head",
    );

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls")
            .header("content-type", "application/json")
            .body_contains("\"access_token\":\"secret-token\"")
            .body_contains("\"title\":\"Use local head\"")
            .body_contains("\"head\":\"feature/local-head\"")
            .body_contains("\"base\":\"develop\"")
            .body_contains("\"body\":\"Built from the local branch\"");
        then.status(201).json_body(pull_request_payload(
            43,
            "Use local head",
            Some("Built from the local branch"),
            "octocat",
            "feature/local-head",
            "develop",
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--title",
            "Use local head",
            "--base",
            "develop",
            "--body",
            "Built from the local branch",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Created pull request #43
repository: octo/demo
head: octo/demo:feature/local-head
base: octo/demo:develop
url: https://gitee.com/octo/demo/pulls/43"
    );

    create_mock.assert_hits(1);
}

#[test]
fn pr_create_reads_body_from_a_file() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("body.md");
    std::fs::write(&body_file, "Generated from a file").unwrap();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls")
            .header("content-type", "application/json")
            .body_contains("\"access_token\":\"secret-token\"")
            .body_contains("\"title\":\"Read body file\"")
            .body_contains("\"head\":\"feature/body-file\"")
            .body_contains("\"base\":\"main\"")
            .body_contains("\"body\":\"Generated from a file\"");
        then.status(201).json_body(pull_request_payload(
            44,
            "Read body file",
            Some("Generated from a file"),
            "octocat",
            "feature/body-file",
            "main",
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/body-file",
            "--base",
            "main",
            "--title",
            "Read body file",
            "--body-file",
            body_file.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 44);
    assert_eq!(body["body"], "Generated from a file");

    create_mock.assert_hits(1);
}

#[test]
fn pr_create_reads_body_from_stdin_via_body_file_dash() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls")
            .header("content-type", "application/json")
            .body_contains("\"access_token\":\"secret-token\"")
            .body_contains("\"title\":\"Read stdin\"")
            .body_contains("\"head\":\"feature/stdin\"")
            .body_contains("\"base\":\"main\"")
            .body_contains("\"body\":\"Generated from stdin\\n\"");
        then.status(201).json_body(pull_request_payload(
            45,
            "Read stdin",
            Some("Generated from stdin\n"),
            "octocat",
            "feature/stdin",
            "main",
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/stdin",
            "--base",
            "main",
            "--title",
            "Read stdin",
            "--body-file",
            "-",
            "--json",
        ])
        .write_stdin("Generated from stdin\n")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 45);
    assert_eq!(body["body"], "Generated from stdin\n");

    create_mock.assert_hits(1);
}

#[test]
fn pr_create_rejects_body_and_body_file_together() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("body.md");
    std::fs::write(&body_file, "Generated from a file").unwrap();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200);
    });

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/v5/repos/octo/demo/pulls");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/body-file",
            "--base",
            "main",
            "--title",
            "Read body file",
            "--body",
            "Generated from a flag",
            "--body-file",
            body_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "provide only one of --body or --body-file"
    );

    repo_mock.assert_hits(0);
    create_mock.assert_hits(0);
}

#[test]
fn pr_create_requires_authentication() {
    let credential_store_dir = tempfile::TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env(
            "GITEE_TEST_CREDENTIAL_STORE_DIR",
            credential_store_dir.path(),
        )
        .env_remove("GITEE_TOKEN")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/no-auth",
            "--base",
            "main",
            "--title",
            "No auth",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr create"
    );
}

#[test]
fn pr_create_fails_when_current_branch_is_not_pushed_to_origin() {
    let repo_dir = git_repo_with_gitee_origin("feature/unpushed");

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--title",
            "Unpushed branch",
            "--base",
            "main",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "git context error: current branch is not pushed to origin"
    );
}

#[test]
fn pr_create_fails_when_current_branch_tracks_a_non_origin_remote() {
    let repo_dir = git_repo_with_gitee_origin("feature/forked");
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

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "create", "--title", "Forked branch", "--base", "main"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "git context error: current branch tracks remote `fork`, expected origin"
    );
}

#[test]
fn pr_create_surfaces_remote_validation_errors_instead_of_auth_failure() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo")
            .query_param("access_token", "secret-token");
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

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/v5/repos/octo/demo/pulls");
        then.status(400).json_body(serde_json::json!({
            "message": "source branch does not exist"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "create",
            "--repo",
            "octo/demo",
            "--head",
            "feature/missing",
            "--title",
            "Example",
            "--body",
            "Body",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "remote request failed (400): source branch does not exist"
    );

    repo_mock.assert_hits(1);
    create_mock.assert_hits(1);
}

fn pull_request_payload(
    number: u64,
    title: &str,
    body: Option<&str>,
    author: &str,
    head_ref: &str,
    base_ref: &str,
) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": title,
        "body": body,
        "html_url": format!("https://gitee.com/octo/demo/pulls/{number}"),
        "draft": false,
        "mergeable": true,
        "created_at": "2026-03-20T09:00:00+08:00",
        "updated_at": "2026-03-20T10:00:00+08:00",
        "merged_at": null,
        "user": {
            "login": author
        },
        "head": {
            "ref": head_ref,
            "sha": "abc123",
            "repo": {
                "full_name": "octo/demo"
            }
        },
        "base": {
            "ref": base_ref,
            "sha": "def456",
            "repo": {
                "full_name": "octo/demo"
            }
        }
    })
}

fn git_repo_with_gitee_origin(branch: &str) -> TempDir {
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
    run_git(
        repo_dir.path(),
        &["remote", "add", "origin", "https://gitee.com/octo/demo.git"],
    );

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
