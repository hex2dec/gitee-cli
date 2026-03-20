use assert_cmd::Command;
use httpmock::Method::POST;
use httpmock::MockServer;
use serde_json::Value;

#[test]
fn auth_login_rejects_equals_syntax_for_long_options() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["auth", "login", "--token=inline-token"])
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
fn pr_view_rejects_a_standalone_double_dash_argument() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["pr", "view", "--", "42"])
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
