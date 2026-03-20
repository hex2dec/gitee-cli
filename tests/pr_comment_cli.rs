use assert_cmd::Command;
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_comment_posts_a_general_comment_from_local_repo_context_in_json_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/comment");

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/42")
            .query_param("access_token", "secret-token");
        then.status(200)
            .json_body(pull_request_payload(42, "feature/comment"));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/42/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Ship+it");
        then.status(201).json_body(serde_json::json!({
            "id": 101,
            "body": "Ship it",
            "html_url": "https://gitee.com/octo/demo/pulls/42#note_101",
            "comment_type": "pr_comment",
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T09:00:00+08:00",
            "user": {
                "login": "octocat"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "comment", "42", "--body", "Ship it", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["id"], 101);
    assert_eq!(body["body"], "Ship it");
    assert_eq!(body["author"], "octocat");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["pull_request"], 42);
    assert_eq!(body["comment_type"], "pr_comment");

    pr_mock.assert_hits(1);
    comment_mock.assert_hits(1);
}

#[test]
fn pr_comment_reads_body_from_a_file_and_renders_text_output() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("comment.md");
    std::fs::write(&body_file, "Looks good to me").unwrap();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/43")
            .query_param("access_token", "secret-token");
        then.status(200)
            .json_body(pull_request_payload(43, "feature/file"));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/43/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Looks+good+to+me");
        then.status(201).json_body(serde_json::json!({
            "id": 102,
            "body": "Looks good to me",
            "html_url": "https://gitee.com/octo/demo/pulls/43#note_102",
            "comment_type": "pr_comment",
            "created_at": "2026-03-20T10:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "user": {
                "login": "octocat"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "comment",
            "43",
            "--repo",
            "octo/demo",
            "--body-file",
            body_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Commented on pull request #43
comment id: 102
repository: octo/demo
author: octocat
url: https://gitee.com/octo/demo/pulls/43#note_102"
    );

    pr_mock.assert_hits(1);
    comment_mock.assert_hits(1);
}

#[test]
fn pr_comment_reads_body_from_stdin_via_body_file_dash() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/44")
            .query_param("access_token", "secret-token");
        then.status(200)
            .json_body(pull_request_payload(44, "feature/stdin"));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/44/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Generated+from+stdin%0A");
        then.status(201).json_body(serde_json::json!({
            "id": 103,
            "body": "Generated from stdin\n",
            "html_url": "https://gitee.com/octo/demo/pulls/44#note_103",
            "comment_type": "pr_comment",
            "created_at": "2026-03-20T11:00:00+08:00",
            "updated_at": "2026-03-20T11:00:00+08:00",
            "user": {
                "login": "octocat"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "comment",
            "44",
            "--repo",
            "octo/demo",
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
    assert_eq!(body["id"], 103);
    assert_eq!(body["body"], "Generated from stdin\n");

    pr_mock.assert_hits(1);
    comment_mock.assert_hits(1);
}

#[test]
fn pr_comment_rejects_body_and_body_file_together() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("comment.md");
    std::fs::write(&body_file, "Looks good to me").unwrap();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/44");
        then.status(200);
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/44/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "comment",
            "44",
            "--repo",
            "octo/demo",
            "--body",
            "Ship it",
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

    pr_mock.assert_hits(0);
    comment_mock.assert_hits(0);
}

#[test]
fn pr_comment_requires_authentication() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env_remove("GITEE_TOKEN")
        .args([
            "pr",
            "comment",
            "42",
            "--repo",
            "octo/demo",
            "--body",
            "Ship it",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr comment"
    );
}

#[test]
fn pr_comment_reports_a_missing_pull_request() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/404")
            .query_param("access_token", "secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

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

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "comment",
            "404",
            "--repo",
            "octo/demo",
            "--body",
            "Missing",
        ])
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
fn pr_comment_rejects_an_empty_body() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "comment",
            "42",
            "--repo",
            "octo/demo",
            "--body",
            "   ",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "comment body cannot be empty"
    );
}

fn pull_request_payload(number: u64, head_ref: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": "PR comment target",
        "body": null,
        "html_url": format!("https://gitee.com/octo/demo/pulls/{number}"),
        "draft": false,
        "mergeable": true,
        "created_at": "2026-03-20T08:00:00+08:00",
        "updated_at": "2026-03-20T08:30:00+08:00",
        "merged_at": null,
        "user": {
            "login": "octocat"
        },
        "head": {
            "ref": head_ref,
            "sha": "abc123",
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

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
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
