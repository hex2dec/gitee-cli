use assert_cmd::Command;
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn auth_login_accepts_equals_syntax_for_long_options() {
    let config_dir = TempDir::new().unwrap();
    let credential_store_dir = TempDir::new().unwrap();
    let server = MockServer::start();

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "inline-token");
        then.status(200).json_body(serde_json::json!({
            "login": "inline-user"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env(
            "GITEE_TEST_CREDENTIAL_STORE_DIR",
            credential_store_dir.path(),
        )
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "login", "--token=inline-token", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["authenticated"], true);
    assert_eq!(body["source"], "keyring");
    assert_eq!(body["username"], "inline-user");

    user_mock.assert_hits(1);
}

#[test]
fn pr_view_accepts_a_standalone_double_dash_before_positionals() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/42");
        then.status(200).json_body(serde_json::json!({
            "number": 42,
            "state": "open",
            "title": "Double dash parsing",
            "body": null,
            "html_url": "https://gitee.com/octo/demo/pulls/42",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-03-20T15:00:00+08:00",
            "updated_at": "2026-03-20T15:30:00+08:00",
            "merged_at": null,
            "user": {
                "login": "octocat"
            },
            "head": {
                "ref": "feature/double-dash",
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
        .args(["pr", "view", "--repo", "octo/demo", "--json", "--", "42"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "Double dash parsing");

    pr_mock.assert_hits(1);
}

#[test]
fn issue_comment_accepts_a_hyphen_prefixed_body_value() {
    let server = MockServer::start();

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/issues/I123/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=--token%3Dabc");
        then.status(201).json_body(serde_json::json!({
            "id": 321,
            "body": "--token=abc",
            "created_at": "2026-03-20T14:00:00Z",
            "updated_at": "2026-03-20T14:01:00Z",
            "user": {
                "login": "parser-check"
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
            "--token=abc",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["body"], "--token=abc");
    assert_eq!(body["id"], 321);

    comment_mock.assert_hits(1);
}
