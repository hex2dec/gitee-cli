use assert_cmd::Command;
use httpmock::Method::{GET, PATCH};
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_edit_updates_title_with_explicit_repo_in_json_output() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/42")
            .query_param("access_token", "secret-token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body_contains("title=Updated+title");
        then.status(200).json_body(pull_request_payload(
            42,
            "Updated title",
            Some("Existing body"),
            "octocat",
            "feature/pr-edit",
            "main",
            false,
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "edit",
            "42",
            "--repo",
            "octo/demo",
            "--title",
            "Updated title",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "Updated title");
    assert_eq!(body["body"], "Existing body");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["head_ref"], "feature/pr-edit");
    assert_eq!(body["base_ref"], "main");
    assert_eq!(body["draft"], false);

    edit_mock.assert_hits(1);
}

#[test]
fn pr_edit_resolves_human_name_remote_to_canonical_private_repo() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:hzw/tip-ucan.git", "feature/human-name");

    let direct_edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/hzw/tip-ucan/pulls/42")
            .query_param("access_token", "secret-token")
            .body_contains("title=Canonical+title");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_list_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user/repos")
            .query_param("access_token", "secret-token");
        then.status(200).json_body(serde_json::json!([
            {
                "full_name": "hzw-dev/tip-ucan",
                "human_name": "hzw/tip-ucan",
                "path": "tip-ucan",
                "html_url": "https://gitee.com/hzw-dev/tip-ucan.git",
                "ssh_url": "git@gitee.com:hzw-dev/tip-ucan.git",
                "fork": false,
                "default_branch": "main"
            }
        ]));
    });

    let canonical_edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/hzw-dev/tip-ucan/pulls/42")
            .query_param("access_token", "secret-token")
            .body_contains("title=Canonical+title");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Canonical title",
            "body": null,
            "html_url": "https://gitee.com/hzw-dev/tip-ucan/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/human-name",
                "sha": "abc123",
                "repo": {
                    "full_name": "hzw-dev/tip-ucan"
                }
            },
            "base": {
                "ref": "main",
                "sha": "def456",
                "repo": {
                    "full_name": "hzw-dev/tip-ucan"
                }
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "edit", "42", "--title", "Canonical title", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["head_repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["base_repository"], "hzw-dev/tip-ucan");

    direct_edit_mock.assert_hits(1);
    repo_list_mock.assert_hits(1);
    canonical_edit_mock.assert_hits(1);
}

#[test]
fn pr_edit_reads_body_from_stdin_marks_ready_and_renders_text_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/edit");

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/44")
            .query_param("access_token", "secret-token")
            .body_contains("body=Generated+from+stdin%0A")
            .body_contains("draft=false");
        then.status(200).json_body(pull_request_payload(
            44,
            "Ready for review",
            Some("Generated from stdin\n"),
            "octocat",
            "feature/edit",
            "main",
            false,
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "edit", "44", "--body-file", "-", "--ready"])
        .write_stdin("Generated from stdin\n")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
#44 Ready for review
state: open
author: octocat
repository: octo/demo
head: octo/demo:feature/edit
base: octo/demo:main
draft: false
mergeable: true
url: https://gitee.com/octo/demo/pulls/44"
    );

    edit_mock.assert_hits(1);
}

#[test]
fn pr_edit_allows_clearing_body_with_an_explicit_empty_string() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/45")
            .query_param("access_token", "secret-token")
            .body_contains("body=");
        then.status(200).json_body(pull_request_payload(
            45,
            "Empty body",
            None,
            "octocat",
            "feature/empty-body",
            "main",
            false,
        ));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "edit",
            "45",
            "--repo",
            "octo/demo",
            "--body",
            "",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 45);
    assert_eq!(body["body"], Value::Null);

    edit_mock.assert_hits(1);
}

#[test]
fn pr_edit_updates_state_to_closed() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/46")
            .query_param("access_token", "secret-token")
            .body_contains("state=closed");
        then.status(200).json_body(serde_json::json!({
            "number": 46,
            "state": "closed",
            "title": "Closed PR",
            "body": null,
            "html_url": "https://gitee.com/octo/demo/pulls/46",
            "draft": false,
            "mergeable": false,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/closed",
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
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "edit",
            "46",
            "--repo",
            "octo/demo",
            "--state",
            "closed",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["state"], "closed");

    edit_mock.assert_hits(1);
}

#[test]
fn pr_edit_requires_authentication() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env_remove("GITEE_TOKEN")
        .args([
            "pr",
            "edit",
            "42",
            "--repo",
            "octo/demo",
            "--title",
            "No auth",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr edit"
    );
}

#[test]
fn pr_edit_requires_at_least_one_mutation_flag() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["pr", "edit", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "pr edit requires at least one of --title, --body, --body-file, --state, --draft, or --ready"
    );
}

#[test]
fn pr_edit_rejects_body_and_body_file_together() {
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("body.md");
    std::fs::write(&body_file, "Generated from a file").unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "pr",
            "edit",
            "42",
            "--repo",
            "octo/demo",
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
}

#[test]
fn pr_edit_rejects_draft_and_ready_together() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "pr",
            "edit",
            "42",
            "--repo",
            "octo/demo",
            "--title",
            "Conflicting draft flags",
            "--draft",
            "--ready",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "provide only one of --draft or --ready"
    );
}

#[test]
fn pr_edit_fails_when_pull_request_is_missing() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
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
            "edit",
            "404",
            "--repo",
            "octo/demo",
            "--title",
            "Missing PR",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "pull request not found"
    );

    edit_mock.assert_hits(1);
    repo_mock.assert_hits(1);
}

#[test]
fn pr_edit_fails_when_repository_is_missing() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/missing/pulls/404")
            .query_param("access_token", "secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/missing")
            .query_param("access_token", "secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "edit",
            "404",
            "--repo",
            "octo/missing",
            "--title",
            "Missing repo",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "repository not found"
    );

    edit_mock.assert_hits(1);
    repo_mock.assert_hits(1);
}

#[test]
fn pr_edit_surfaces_remote_validation_errors() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/47")
            .query_param("access_token", "secret-token");
        then.status(400).json_body(serde_json::json!({
            "message": "state transition is not allowed"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "edit",
            "47",
            "--repo",
            "octo/demo",
            "--state",
            "closed",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "remote request failed (400): state transition is not allowed"
    );

    edit_mock.assert_hits(1);
}

fn pull_request_payload(
    number: u64,
    title: &str,
    body: Option<&str>,
    author: &str,
    head_ref: &str,
    base_ref: &str,
    draft: bool,
) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": title,
        "body": body,
        "html_url": format!("https://gitee.com/octo/demo/pulls/{number}"),
        "draft": draft,
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

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
    let repo_dir = TempDir::new().unwrap();

    run_git(repo_dir.path(), &["init"]);
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
