use assert_cmd::Command;
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn issue_list_uses_local_repo_context_and_reports_stable_json_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/issues");

    let issues_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/issues")
            .query_param("state", "closed")
            .query_param("q", "panic")
            .query_param("page", "2")
            .query_param("per_page", "5");
        then.status(200).json_body(serde_json::json!([
            {
                "number": "I123",
                "title": "Fix panic in issue sync",
                "state": "closed",
                "body": "panic body",
                "comments": 2,
                "html_url": "https://gitee.com/octo/demo/issues/I123",
                "created_at": "2026-03-20T10:00:00Z",
                "updated_at": "2026-03-20T12:00:00Z",
                "user": {
                    "login": "alice"
                }
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "issue",
            "list",
            "--state",
            "closed",
            "--search",
            "panic",
            "--page",
            "2",
            "--per-page",
            "5",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "local");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["state"], "closed");
    assert_eq!(body["search"], "panic");
    assert_eq!(body["page"], 2);
    assert_eq!(body["per_page"], 5);
    assert_eq!(body["issues"][0]["number"], "I123");
    assert_eq!(body["issues"][0]["title"], "Fix panic in issue sync");
    assert_eq!(body["issues"][0]["state"], "closed");
    assert_eq!(body["issues"][0]["author"], "alice");
    assert_eq!(body["issues"][0]["comments"], 2);
    assert_eq!(
        body["issues"][0]["html_url"],
        "https://gitee.com/octo/demo/issues/I123"
    );

    issues_mock.assert_hits(1);
}

#[test]
fn issue_create_uses_local_repo_context_and_reports_stable_json_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/issues");

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Add+issue+create")
            .body_contains("body=Creates+an+issue+from+the+CLI");
        then.status(201).json_body(serde_json::json!({
            "number": "I124",
            "title": "Add issue create",
            "state": "open",
            "body": "Creates an issue from the CLI",
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I124",
            "created_at": "2026-03-20T14:00:00Z",
            "updated_at": "2026-03-20T14:00:00Z",
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
            "create",
            "--title",
            "Add issue create",
            "--body",
            "Creates an issue from the CLI",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "local");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["number"], "I124");
    assert_eq!(body["title"], "Add issue create");
    assert_eq!(body["state"], "open");
    assert_eq!(body["author"], "alice");
    assert_eq!(body["body"], "Creates an issue from the CLI");
    assert_eq!(body["comments"], 0);
    assert_eq!(body["html_url"], "https://gitee.com/octo/demo/issues/I124");
    assert_eq!(body["created_at"], "2026-03-20T14:00:00Z");
    assert_eq!(body["updated_at"], "2026-03-20T14:00:00Z");

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_uses_explicit_repo_and_renders_text_output() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Use+explicit+repo")
            .body_contains("body=Created+with+an+explicit+repo");
        then.status(201).json_body(serde_json::json!({
            "number": "I125",
            "title": "Use explicit repo",
            "state": "open",
            "body": "Created with an explicit repo",
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I125",
            "created_at": "2026-03-20T14:30:00Z",
            "updated_at": "2026-03-20T14:30:00Z",
            "user": {
                "login": "bob"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "create",
            "--repo",
            "octo/demo",
            "--title",
            "Use explicit repo",
            "--body",
            "Created with an explicit repo",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Created issue I125
repository: octo/demo
title: Use explicit repo
state: open
author: bob
source: explicit
url: https://gitee.com/octo/demo/issues/I125"
    );

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_reads_body_from_a_file() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("body.md");
    std::fs::write(&body_file, "Generated from a file").unwrap();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Read+issue+body+file")
            .body_contains("body=Generated+from+a+file");
        then.status(201).json_body(serde_json::json!({
            "number": "I126",
            "title": "Read issue body file",
            "state": "open",
            "body": "Generated from a file",
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I126",
            "created_at": "2026-03-20T15:00:00Z",
            "updated_at": "2026-03-20T15:00:00Z",
            "user": {
                "login": "carol"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "create",
            "--repo",
            "octo/demo",
            "--title",
            "Read issue body file",
            "--body-file",
            body_file.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], "I126");
    assert_eq!(body["body"], "Generated from a file");

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_reads_body_from_stdin_via_body_file_dash() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Read+issue+stdin")
            .body_contains("body=Generated+from+stdin%0A");
        then.status(201).json_body(serde_json::json!({
            "number": "I127",
            "title": "Read issue stdin",
            "state": "open",
            "body": "Generated from stdin\n",
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I127",
            "created_at": "2026-03-20T15:30:00Z",
            "updated_at": "2026-03-20T15:30:00Z",
            "user": {
                "login": "dora"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "create",
            "--repo",
            "octo/demo",
            "--title",
            "Read issue stdin",
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
    assert_eq!(body["number"], "I127");
    assert_eq!(body["body"], "Generated from stdin\n");

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_surfaces_remote_validation_errors_instead_of_auth_failure() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Bad");
        then.status(400).json_body(serde_json::json!({
            "message": "title is too short"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["issue", "create", "--repo", "octo/demo", "--title", "Bad"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "remote request failed (400): title is too short"
    );

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_requires_authentication() {
    let config_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args([
            "issue",
            "create",
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
        "authentication required for issue create"
    );
}

#[test]
fn issue_create_fails_when_authentication_is_rejected() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=bad-token")
            .body_contains("repo=demo")
            .body_contains("title=Needs+auth");
        then.status(401).json_body(serde_json::json!({
            "message": "Unauthorized"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "bad-token")
        .args([
            "issue",
            "create",
            "--repo",
            "octo/demo",
            "--title",
            "Needs auth",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication failed"
    );

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_fails_when_repository_is_missing() {
    let server = MockServer::start();

    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Missing+repo");
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
            "create",
            "--repo",
            "octo/demo",
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

    create_mock.assert_hits(1);
}

#[test]
fn issue_create_rejects_missing_title() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["issue", "create", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue create requires --title"
    );
}

#[test]
fn issue_create_rejects_body_and_body_file_together() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_file = temp_dir.path().join("body.md");
    std::fs::write(&body_file, "Generated from a file").unwrap();

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/v5/repos/octo/issues");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "create",
            "--repo",
            "octo/demo",
            "--title",
            "Conflict body inputs",
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

    create_mock.assert_hits(0);
}

#[test]
fn issue_create_fails_when_not_inside_a_git_repository() {
    let working_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_TOKEN", "secret-token")
        .args(["issue", "create", "--title", "hello from local repo"])
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
fn issue_comment_posts_a_reply_from_a_direct_body_flag() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Thanks+for+the+detailed+report");
        then.status(201).json_body(serde_json::json!({
            "id": 99,
            "body": "Thanks for the detailed report",
            "created_at": "2026-03-20T12:30:00Z",
            "updated_at": "2026-03-20T12:31:00Z",
            "user": {
                "login": "carol"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body",
            "Thanks for the detailed report",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "explicit");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["number"], "I123");
    assert_eq!(body["id"], 99);
    assert_eq!(body["author"], "carol");
    assert_eq!(body["body"], "Thanks for the detailed report");
    assert_eq!(body["created_at"], "2026-03-20T12:30:00Z");
    assert_eq!(body["updated_at"], "2026-03-20T12:31:00Z");

    comment_mock.assert_hits(1);
}

#[test]
fn issue_comment_supports_body_file_input_and_stable_text_output() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_path = temp_dir.path().join("comment.txt");
    std::fs::write(&body_path, "Posted from file").unwrap();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Posted+from+file");
        then.status(201).json_body(serde_json::json!({
            "id": 100,
            "body": "Posted from file",
            "created_at": "2026-03-20T13:00:00Z",
            "updated_at": "2026-03-20T13:01:00Z",
            "user": {
                "login": "dora"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
octo/demo#I123
comment id: 100
author: dora
created at: 2026-03-20T13:00:00Z
updated at: 2026-03-20T13:01:00Z
source: explicit
body:
Posted from file"
    );

    comment_mock.assert_hits(1);
}

#[test]
fn issue_comment_supports_stdin_body_input_via_body_file_dash() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Posted+from+stdin");
        then.status(201).json_body(serde_json::json!({
            "id": 101,
            "body": "Posted from stdin",
            "created_at": "2026-03-20T13:30:00Z",
            "updated_at": "2026-03-20T13:31:00Z",
            "user": {
                "login": "erin"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .write_stdin("Posted from stdin")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            "-",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], "I123");
    assert_eq!(body["id"], 101);
    assert_eq!(body["author"], "erin");
    assert_eq!(body["body"], "Posted from stdin");

    comment_mock.assert_hits(1);
}

#[test]
fn issue_comment_fails_when_issue_is_missing() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I404/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Missing+issue+comment");
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
            "comment",
            "I404",
            "--repo",
            "octo/demo",
            "--body",
            "Missing issue comment",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue not found"
    );

    comment_mock.assert_hits(1);
}

#[test]
fn issue_comment_fails_when_authentication_is_rejected() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("access_token", "bad-token")
            .body_contains("body=Needs+auth");
        then.status(401).json_body(serde_json::json!({
            "message": "Unauthorized"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "bad-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body",
            "Needs auth",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication failed"
    );

    comment_mock.assert_hits(1);
}

#[test]
fn issue_comment_rejects_missing_body_input() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["issue", "comment", "I123", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "issue comment requires --body or --body-file"
    );
}

#[test]
fn issue_comment_rejects_body_and_body_file_together() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_path = temp_dir.path().join("comment.txt");
    std::fs::write(&body_path, "Posted from file").unwrap();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body",
            "Posted from flag",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "provide only one of --body or --body-file"
    );

    comment_mock.assert_hits(0);
}

#[test]
fn issue_comment_rejects_removed_body_stdin_flag_as_unsupported() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .write_stdin("Posted from stdin")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-stdin",
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
fn issue_comment_rejects_whitespace_only_flag_body() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body",
            "  \n\t  ",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "comment body cannot be empty"
    );

    comment_mock.assert_hits(0);
}

#[test]
fn issue_comment_rejects_whitespace_only_body_file() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_path = temp_dir.path().join("comment.txt");
    std::fs::write(&body_path, " \n\t ").unwrap();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "comment body cannot be empty"
    );

    comment_mock.assert_hits(0);
}

#[test]
fn issue_comment_rejects_whitespace_only_stdin_body_via_body_file_dash() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .write_stdin(" \n\t ")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            "-",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "comment body cannot be empty"
    );

    comment_mock.assert_hits(0);
}

#[test]
fn issue_comment_rejects_missing_body_file() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let missing_path = temp_dir.path().join("missing-comment.txt");

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            missing_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .trim()
            .starts_with("failed to read comment body file: ")
    );

    comment_mock.assert_hits(0);
}

#[cfg(unix)]
#[test]
fn issue_comment_rejects_unreadable_body_file() {
    let server = MockServer::start();
    let temp_dir = TempDir::new().unwrap();
    let body_path = temp_dir.path().join("comment.txt");
    std::fs::write(&body_path, "Posted from file").unwrap();

    let mut permissions = std::fs::metadata(&body_path).unwrap().permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&body_path, permissions).unwrap();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--repo",
            "octo/demo",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .trim()
            .starts_with("failed to read comment body file: ")
    );

    comment_mock.assert_hits(0);
}

#[test]
fn issue_comment_fails_when_not_inside_a_git_repository() {
    let working_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "issue",
            "comment",
            "I123",
            "--body",
            "hello from local repo",
        ])
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
fn issue_view_skips_comment_history_by_default_and_reports_stable_json_output() {
    let server = MockServer::start();

    let issue_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/issues/I123");
        then.status(200).json_body(serde_json::json!({
            "number": "I123",
            "title": "Fix issue sync panic",
            "state": "open",
            "body": "full issue body",
            "comments": 3,
            "html_url": "https://gitee.com/octo/demo/issues/I123",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T12:00:00Z",
            "user": {
                "login": "bob"
            }
        }));
    });
    let comments_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/issues/I123/comments");
        then.status(200).json_body(serde_json::json!([]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["issue", "view", "I123", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "explicit");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["number"], "I123");
    assert_eq!(body["title"], "Fix issue sync panic");
    assert_eq!(body["state"], "open");
    assert_eq!(body["author"], "bob");
    assert_eq!(body["body"], "full issue body");
    assert_eq!(body["comments_count"], 3);
    assert_eq!(body["comments_included"], false);
    assert_eq!(body["comments_page"], Value::Null);
    assert_eq!(body["comments_per_page"], Value::Null);
    assert_eq!(body["comments"], Value::Null);

    issue_mock.assert_hits(1);
    comments_mock.assert_hits(0);
}

#[test]
fn issue_view_supports_gh_style_json_field_selection() {
    let server = MockServer::start();

    let issue_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/issues/I123");
        then.status(200).json_body(serde_json::json!({
            "number": "I123",
            "title": "Fix issue sync panic",
            "state": "open",
            "body": "full issue body",
            "comments": 3,
            "html_url": "https://gitee.com/octo/demo/issues/I123",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T12:00:00Z",
            "user": {
                "login": "bob"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "issue",
            "view",
            "I123",
            "--repo",
            "octo/demo",
            "--json",
            "number,title,url",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], "I123");
    assert_eq!(body["title"], "Fix issue sync panic");
    assert_eq!(body["url"], "https://gitee.com/octo/demo/issues/I123");

    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 3);
    assert!(!object.contains_key("html_url"));
    assert!(!object.contains_key("body"));

    issue_mock.assert_hits(1);
}

#[test]
fn issue_view_includes_paginated_comments_when_requested() {
    let server = MockServer::start();

    let issue_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/issues/I123");
        then.status(200).json_body(serde_json::json!({
            "number": "I123",
            "title": "Fix issue sync panic",
            "state": "open",
            "body": "full issue body",
            "comments": 3,
            "html_url": "https://gitee.com/octo/demo/issues/I123",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T12:00:00Z",
            "user": {
                "login": "bob"
            }
        }));
    });
    let comments_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("page", "2")
            .query_param("per_page", "1");
        then.status(200).json_body(serde_json::json!([
            {
                "id": 99,
                "body": "Please add a regression test",
                "created_at": "2026-03-20T12:30:00Z",
                "updated_at": "2026-03-20T12:31:00Z",
                "user": {
                    "login": "carol"
                }
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "issue",
            "view",
            "I123",
            "--repo",
            "octo/demo",
            "--comments",
            "--page",
            "2",
            "--per-page",
            "1",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["comments_included"], true);
    assert_eq!(body["comments_page"], 2);
    assert_eq!(body["comments_per_page"], 1);
    assert_eq!(body["comments"][0]["id"], 99);
    assert_eq!(body["comments"][0]["author"], "carol");
    assert_eq!(body["comments"][0]["body"], "Please add a regression test");
    assert_eq!(body["comments"][0]["created_at"], "2026-03-20T12:30:00Z");
    assert_eq!(body["comments"][0]["updated_at"], "2026-03-20T12:31:00Z");

    issue_mock.assert_hits(1);
    comments_mock.assert_hits(1);
}

#[test]
fn issue_list_renders_stable_text_output() {
    let server = MockServer::start();

    let issues_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/issues")
            .query_param("state", "open")
            .query_param("page", "1")
            .query_param("per_page", "20");
        then.status(200).json_body(serde_json::json!([
            {
                "number": "I123",
                "title": "Fix panic in issue sync",
                "state": "open",
                "body": "panic body",
                "comments": 2,
                "html_url": "https://gitee.com/octo/demo/issues/I123",
                "created_at": "2026-03-20T10:00:00Z",
                "updated_at": "2026-03-20T12:00:00Z",
                "user": {
                    "login": "alice"
                }
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["issue", "list", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
octo/demo issues
state: open
search: (none)
page: 1
per page: 20
source: explicit
I123 | open | alice | comments: 2 | Fix panic in issue sync"
    );

    issues_mock.assert_hits(1);
}

#[test]
fn issue_list_rejects_an_invalid_state_filter() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["issue", "list", "--repo", "octo/demo", "--state", "invalid"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "invalid value for --state: expected open, closed, or all"
    );
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
