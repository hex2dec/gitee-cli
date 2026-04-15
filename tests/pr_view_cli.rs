use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_view_supports_explicit_repo_in_json_output() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Fix pull request rendering",
            "body": "Adds stable PR rendering",
            "html_url": "https://gitee.com/octo/demo/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/pr-view",
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
        .args(["pr", "view", "42", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["state"], "open");
    assert_eq!(body["title"], "Fix pull request rendering");
    assert_eq!(body["body"], "Adds stable PR rendering");
    assert_eq!(body["author"], "octocat");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["head_ref"], "feature/pr-view");
    assert_eq!(body["head_sha"], "abc123");
    assert_eq!(body["head_repository"], "octo/demo");
    assert_eq!(body["base_ref"], "main");
    assert_eq!(body["base_sha"], "def456");
    assert_eq!(body["base_repository"], "octo/demo");
    assert_eq!(body["draft"], false);
    assert_eq!(body["mergeable"], true);
    assert_eq!(body["html_url"], "https://gitee.com/octo/demo/pulls/42");
    assert_eq!(body["created_at"], "2026-03-20T09:00:00+08:00");
    assert_eq!(body["updated_at"], "2026-03-20T10:00:00+08:00");
    assert_eq!(body["merged_at"], Value::Null);

    pr_mock.assert_hits(1);
}

#[test]
fn pr_view_supports_gh_style_json_field_selection() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Fix pull request rendering",
            "body": "Adds stable PR rendering",
            "html_url": "https://gitee.com/octo/demo/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/pr-view",
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
        .args([
            "pr",
            "view",
            "42",
            "--repo",
            "octo/demo",
            "--json",
            "number,url,title",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "Fix pull request rendering");
    assert_eq!(body["url"], "https://gitee.com/octo/demo/pulls/42");

    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 3);
    assert!(!object.contains_key("html_url"));

    pr_mock.assert_hits(1);
}

#[test]
fn pr_view_supports_extended_gh_style_json_fields() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Fix pull request rendering",
            "body": "Adds stable PR rendering",
            "html_url": "https://gitee.com/octo/demo/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/pr-view",
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
        .args([
            "pr",
            "view",
            "42",
            "--repo",
            "octo/demo",
            "--json",
            "number,title,url,state,body,createdAt,updatedAt,isDraft,mergeable,headRefName,headRefOid,baseRefName,baseRefOid",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "Fix pull request rendering");
    assert_eq!(body["url"], "https://gitee.com/octo/demo/pulls/42");
    assert_eq!(body["state"], "open");
    assert_eq!(body["body"], "Adds stable PR rendering");
    assert_eq!(body["createdAt"], "2026-03-20T09:00:00+08:00");
    assert_eq!(body["updatedAt"], "2026-03-20T10:00:00+08:00");
    assert_eq!(body["isDraft"], false);
    assert_eq!(body["mergeable"], true);
    assert_eq!(body["headRefName"], "feature/pr-view");
    assert_eq!(body["headRefOid"], "abc123");
    assert_eq!(body["baseRefName"], "main");
    assert_eq!(body["baseRefOid"], "def456");

    pr_mock.assert_hits(1);
}

#[test]
fn pr_view_rejects_unknown_json_fields_with_a_specific_usage_error() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "pr",
            "view",
            "42",
            "--repo",
            "octo/demo",
            "--json",
            "number,mergeStateStatus",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "unknown JSON field for pr view: mergeStateStatus"
    );
}

#[test]
fn pr_view_resolves_human_name_remote_to_canonical_private_repo() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:hzw/tip-ucan.git", "feature/human-name");

    let direct_lookup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/hzw/tip-ucan/pulls/42")
            .query_param("access_token", "secret-token");
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

    let canonical_pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/hzw-dev/tip-ucan/pulls/42")
            .query_param("access_token", "secret-token");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Canonical PR",
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
        .args(["pr", "view", "42", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["head_repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["base_repository"], "hzw-dev/tip-ucan");

    direct_lookup_mock.assert_hits(1);
    repo_list_mock.assert_hits(1);
    canonical_pr_mock.assert_hits(1);
}

#[test]
fn pr_view_renders_stable_text_output() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Fix pull request rendering",
            "body": null,
            "html_url": "https://gitee.com/octo/demo/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T10:00:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/pr-view",
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
        .args(["pr", "view", "42", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
#42 Fix pull request rendering
state: open
author: octocat
repository: octo/demo
head: octo/demo:feature/pr-view
base: octo/demo:main
draft: false
mergeable: true
url: https://gitee.com/octo/demo/pulls/42"
    );

    pr_mock.assert_hits(1);
}

#[test]
fn pr_view_fails_when_pull_request_is_missing() {
    let server = MockServer::start();

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
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "view", "404", "--repo", "octo/demo"])
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
fn pr_view_fails_when_repository_is_missing() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/missing/pulls/404");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/missing");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "view", "404", "--repo", "octo/missing"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "repository not found"
    );

    pr_mock.assert_hits(1);
    repo_mock.assert_hits(1);
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
