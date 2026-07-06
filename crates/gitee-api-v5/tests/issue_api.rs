mod common;

use common::client_for;
use gitee_api_v5::{CreateIssue, IssueError, IssueListOptions};
use httpmock::Method::{GET, POST};
use httpmock::MockServer;

#[test]
fn list_repository_issues_sends_filters_and_parses_results() {
    let server = MockServer::start();
    let issues_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/issues")
            .query_param("state", "closed")
            .query_param("q", "panic")
            .query_param("page", "2")
            .query_param("per_page", "5")
            .query_param("access_token", "secret-token");
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

    let issues = client_for(&server)
        .list_repository_issues(
            "octo",
            "demo",
            Some("secret-token"),
            IssueListOptions {
                state: "closed",
                search: Some("panic"),
                page: 2,
                per_page: 5,
            },
        )
        .expect("issue list should parse");

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].number, "I123");
    assert_eq!(issues[0].title, "Fix panic in issue sync");
    assert_eq!(
        issues[0].user.as_ref().map(|user| user.login.as_str()),
        Some("alice")
    );
    issues_mock.assert_hits(1);
}

#[test]
fn create_issue_sends_form_body_and_parses_response() {
    let server = MockServer::start();
    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/issues")
            .body_contains("access_token=secret-token")
            .body_contains("repo=demo")
            .body_contains("title=Add+crate+tests")
            .body_contains("body=Public+API+coverage");
        then.status(201).json_body(serde_json::json!({
            "number": "I124",
            "title": "Add crate tests",
            "state": "open",
            "body": "Public API coverage",
            "comments": 0,
            "html_url": "https://gitee.com/octo/demo/issues/I124",
            "created_at": "2026-03-20T14:00:00Z",
            "updated_at": "2026-03-20T14:00:00Z",
            "user": {
                "login": "alice"
            }
        }));
    });

    let issue = client_for(&server)
        .create_issue(
            "octo",
            "secret-token",
            &CreateIssue {
                repo: "demo",
                title: "Add crate tests",
                body: Some("Public API coverage"),
            },
        )
        .expect("created issue should parse");

    assert_eq!(issue.number, "I124");
    assert_eq!(issue.body.as_deref(), Some("Public API coverage"));
    create_mock.assert_hits(1);
}

#[test]
fn create_issue_preserves_api_error_message() {
    let server = MockServer::start();
    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/v5/repos/octo/issues");
        then.status(422).json_body(serde_json::json!({
            "message": "title cannot be blank"
        }));
    });

    let result = client_for(&server).create_issue(
        "octo",
        "secret-token",
        &CreateIssue {
            repo: "demo",
            title: "",
            body: None,
        },
    );

    match result {
        Err(IssueError::UnexpectedStatusWithMessage(422, message)) => {
            assert_eq!(message, "title cannot be blank");
        }
        _ => panic!("expected 422 API message"),
    }
    create_mock.assert_hits(1);
}
