use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_status_summarizes_current_branch_and_current_user_in_json_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/status");

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "secret-token");
        then.status(200).json_body(serde_json::json!({
            "login": "octocat"
        }));
    });

    let current_branch_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("state", "open")
            .query_param("head", "feature/status")
            .query_param("per_page", "10");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                42,
                "Branch PR",
                "maintainer",
                "feature/status",
                "main"
            )]));
    });

    let authored_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("state", "open")
            .query_param("author", "octocat")
            .query_param("per_page", "10");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                43,
                "Authored PR",
                "octocat",
                "feature/authored",
                "main"
            )]));
    });

    let assigned_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("state", "open")
            .query_param("assignee", "octocat")
            .query_param("per_page", "10");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                44,
                "Assigned PR",
                "teammate",
                "feature/assigned",
                "main"
            )]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "status", "--state", "open", "--limit", "10", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["source"], "local");
    assert_eq!(body["current_user"], "octocat");
    assert_eq!(body["current_branch"], "feature/status");
    assert_eq!(body["current_branch_prs"][0]["number"], 42);
    assert_eq!(body["authored_prs"][0]["number"], 43);
    assert_eq!(body["assigned_prs"][0]["number"], 44);

    user_mock.assert_hits(1);
    current_branch_mock.assert_hits(1);
    authored_mock.assert_hits(1);
    assigned_mock.assert_hits(1);
}

#[test]
fn pr_status_requires_authentication() {
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/status");

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env_remove("GITEE_TOKEN")
        .args(["pr", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr status"
    );
}

#[test]
fn pr_status_supports_default_text_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/status");

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "secret-token");
        then.status(200).json_body(serde_json::json!({
            "login": "octocat"
        }));
    });

    let current_branch_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("head", "feature/status")
            .query_param("per_page", "30");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                42,
                "Branch PR",
                "maintainer",
                "feature/status",
                "main"
            )]));
    });

    let authored_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("author", "octocat")
            .query_param("per_page", "30");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                43,
                "Authored PR",
                "octocat",
                "feature/authored",
                "main"
            )]));
    });

    let assigned_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("assignee", "octocat")
            .query_param("per_page", "30");
        then.status(200)
            .json_body(serde_json::json!([pull_request_payload(
                44,
                "Assigned PR",
                "teammate",
                "feature/assigned",
                "main"
            )]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "status"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Current user: octocat
Current branch: feature/status

Current branch
#42 open Branch PR (maintainer)

Authored by you
#43 open Authored PR (octocat)

Assigned to you
#44 open Assigned PR (teammate)"
    );

    user_mock.assert_hits(1);
    current_branch_mock.assert_hits(1);
    authored_mock.assert_hits(1);
    assigned_mock.assert_hits(1);
}

fn pull_request_payload(
    number: u64,
    title: &str,
    author: &str,
    head_ref: &str,
    base_ref: &str,
) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": title,
        "body": null,
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
