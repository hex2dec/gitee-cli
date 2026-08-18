use assert_cmd::Command;
use httpmock::Method::PATCH;
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn issue_edit_updates_title_with_explicit_repo_and_selected_json_fields() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/issues/I123")
            .header("authorization", "Bearer secret-token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body_contains("repo=demo")
            .body_contains("title=Updated+title");
        then.status(200).json_body(serde_json::json!({
            "number": "I123",
            "title": "Updated title",
            "state": "open",
            "body": "Existing body",
            "comments": 2,
            "html_url": "https://gitee.com/octo/demo/issues/I123",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T12:30:00Z",
            "user": {
                "login": "alice"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "edit",
            "I123",
            "--repo",
            "octo/demo",
            "--title",
            "Updated title",
            "--json",
            "number,title,url",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], "I123");
    assert_eq!(body["title"], "Updated title");
    assert_eq!(body["url"], "https://gitee.com/octo/demo/issues/I123");

    edit_mock.assert_hits(1);
}

#[test]
fn issue_edit_reads_body_from_stdin_updates_state_and_renders_text_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/issue-edit");

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/issues/I124")
            .header("authorization", "Bearer secret-token")
            .body_contains("repo=demo")
            .body_contains("body=Generated+from+stdin%0A")
            .body_contains("state=closed");
        then.status(200).json_body(serde_json::json!({
            "number": "I124",
            "title": "Close completed issue",
            "state": "closed",
            "body": "Generated from stdin\n",
            "comments": 1,
            "html_url": "https://gitee.com/octo/demo/issues/I124",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T13:00:00Z",
            "user": {
                "login": "alice"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "edit",
            "I124",
            "--body-file",
            "-",
            "--state",
            "closed",
        ])
        .write_stdin("Generated from stdin\n")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
octo/demo#I124
title: Close completed issue
state: closed
author: alice
comments: 1
created at: 2026-03-20T10:00:00Z
updated at: 2026-03-20T13:00:00Z
html url: https://gitee.com/octo/demo/issues/I124
source: local
comments included: false
body:
Generated from stdin"
    );

    edit_mock.assert_hits(1);
}

#[test]
fn issue_edit_allows_clearing_body_with_an_explicit_empty_string() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/issues/I125")
            .header("authorization", "Bearer secret-token")
            .body_contains("repo=demo")
            .body_contains("body=");
        then.status(200).json_body(serde_json::json!({
            "number": "I125",
            "title": "Clear body",
            "state": "open",
            "body": null,
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I125",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T13:30:00Z",
            "user": {
                "login": "alice"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "edit",
            "I125",
            "--repo",
            "octo/demo",
            "--body",
            "",
            "--json",
            "number,body",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], "I125");
    assert_eq!(body["body"], "");

    edit_mock.assert_hits(1);
}

#[test]
fn issue_edit_requires_authentication() {
    let config_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args([
            "issue",
            "edit",
            "I123",
            "--repo",
            "octo/demo",
            "--title",
            "Updated title",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for issue edit"
    );
}

#[test]
fn issue_edit_requires_at_least_one_mutation_flag() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["issue", "edit", "I123", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue edit requires at least one of --title, --body, --body-file, or --state"
    );
}

#[test]
fn issue_edit_rejects_multiple_issue_numbers() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "issue",
            "edit",
            "I123",
            "I124",
            "--repo",
            "octo/demo",
            "--title",
            "Updated title",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue edit accepts exactly one issue number"
    );
}

#[test]
fn issue_edit_rejects_body_and_body_file_together() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "issue",
            "edit",
            "I123",
            "--repo",
            "octo/demo",
            "--body",
            "Inline body",
            "--body-file",
            "body.md",
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
fn issue_edit_rejects_invalid_state() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "issue",
            "edit",
            "I123",
            "--repo",
            "octo/demo",
            "--state",
            "progressing",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "invalid value for --state: expected open or closed"
    );
}

#[test]
fn issue_edit_does_not_register_unimplemented_gh_flags() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "issue",
            "edit",
            "I123",
            "--repo",
            "octo/demo",
            "--add-label",
            "bug",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "unsupported command"
    );
}

#[test]
fn issue_edit_fails_when_issue_is_missing() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/issues/I404")
            .header("authorization", "Bearer secret-token")
            .body_contains("repo=demo");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "edit",
            "I404",
            "--repo",
            "octo/demo",
            "--title",
            "Missing issue",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue not found"
    );

    edit_mock.assert_hits(1);
}

#[test]
fn issue_edit_surfaces_remote_validation_errors() {
    let server = MockServer::start();

    let edit_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/issues/I126")
            .header("authorization", "Bearer secret-token")
            .body_contains("repo=demo");
        then.status(400).json_body(serde_json::json!({
            "message": "state transition is not allowed"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "edit",
            "I126",
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

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
    let repo_dir = TempDir::new().unwrap();

    run_git(repo_dir.path(), &["init"]);
    run_git(repo_dir.path(), &["checkout", "-b", branch]);
    run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);

    repo_dir
}

fn run_git(repo_dir: &Path, args: &[&str]) {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(repo_dir)
        .status()
        .unwrap();

    assert!(
        status.success(),
        "git command failed: git {}",
        args.join(" ")
    );
}
