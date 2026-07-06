mod common;

use common::client_for;
use gitee_api_v5::{
    CreatePullRequest, CreatePullRequestComment, MergePullRequest, PullRequestError,
    PullRequestListFilters, UpdatePullRequest,
};
use httpmock::Method::{GET, PATCH, POST, PUT};
use httpmock::MockServer;

fn pull_request_payload(number: u64) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": "Improve API tests",
        "body": "Adds crate-level coverage",
        "html_url": format!("https://gitee.com/octo/demo/pulls/{number}"),
        "draft": false,
        "mergeable": true,
        "created_at": "2026-03-20T09:00:00+08:00",
        "updated_at": "2026-03-20T10:00:00+08:00",
        "merged_at": null,
        "user": {
            "login": "octocat"
        },
        "head": {
            "ref": "feature/api-tests",
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

#[test]
fn fetch_pull_requests_sends_filters_and_parses_results() {
    let server = MockServer::start();
    let pulls_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("access_token", "secret-token")
            .query_param("state", "open")
            .query_param("author", "octocat")
            .query_param("assignee", "reviewer")
            .query_param("base", "main")
            .query_param("head", "feature/api-tests")
            .query_param("per_page", "2");
        then.status(200).json_body(serde_json::json!([
            pull_request_payload(42),
            pull_request_payload(43)
        ]));
    });

    let pulls = client_for(&server)
        .fetch_pull_requests(
            "octo",
            "demo",
            &PullRequestListFilters {
                state: Some("open".to_string()),
                author: Some("octocat".to_string()),
                assignee: Some("reviewer".to_string()),
                base: Some("main".to_string()),
                head: Some("feature/api-tests".to_string()),
                limit: 2,
            },
            Some("secret-token"),
        )
        .expect("pull request list should parse");

    assert_eq!(pulls.len(), 2);
    assert_eq!(pulls[0].number, 42);
    assert_eq!(pulls[0].head.branch, "feature/api-tests");
    assert_eq!(pulls[0].base.branch, "main");
    pulls_mock.assert_hits(1);
}

#[test]
fn create_pull_request_sends_json_body_and_parses_response() {
    let server = MockServer::start();
    let create_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls")
            .json_body(serde_json::json!({
                "access_token": "secret-token",
                "title": "Improve API tests",
                "head": "feature/api-tests",
                "base": "main",
                "body": "Adds crate-level coverage"
            }));
        then.status(201).json_body(pull_request_payload(42));
    });

    let pull = client_for(&server)
        .create_pull_request(
            "octo",
            "demo",
            "secret-token",
            &CreatePullRequest {
                title: "Improve API tests",
                head: "feature/api-tests",
                base: "main",
                body: Some("Adds crate-level coverage"),
            },
        )
        .expect("created pull request should parse");

    assert_eq!(pull.number, 42);
    assert_eq!(pull.title, "Improve API tests");
    create_mock.assert_hits(1);
}

#[test]
fn update_pull_request_sends_form_fields_and_parses_response() {
    let server = MockServer::start();
    let update_mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/v5/repos/octo/demo/pulls/42")
            .query_param("access_token", "secret-token")
            .body_contains("access_token=secret-token")
            .body_contains("title=Updated+title")
            .body_contains("body=Updated+body")
            .body_contains("state=closed")
            .body_contains("draft=true");
        then.status(200).json_body(pull_request_payload(42));
    });

    let pull = client_for(&server)
        .update_pull_request(
            "octo",
            "demo",
            42,
            "secret-token",
            &UpdatePullRequest {
                title: Some("Updated title"),
                body: Some("Updated body"),
                state: Some("closed"),
                draft: Some(true),
            },
        )
        .expect("updated pull request should parse");

    assert_eq!(pull.number, 42);
    update_mock.assert_hits(1);
}

#[test]
fn create_pull_request_comment_sends_token_query_and_form_body() {
    let server = MockServer::start();
    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/42/comments")
            .query_param("access_token", "secret-token")
            .body_contains("body=Looks+good");
        then.status(201).json_body(serde_json::json!({
            "id": 7,
            "body": "Looks good",
            "html_url": "https://gitee.com/octo/demo/pulls/42#note_7",
            "created_at": "2026-03-20T09:00:00+08:00",
            "updated_at": "2026-03-20T09:00:00+08:00",
            "user": {
                "login": "reviewer"
            }
        }));
    });

    let comment = client_for(&server)
        .create_pull_request_comment(
            "octo",
            "demo",
            42,
            "secret-token",
            &CreatePullRequestComment { body: "Looks good" },
        )
        .expect("created pull request comment should parse");

    assert_eq!(comment.id, 7);
    assert_eq!(comment.body, "Looks good");
    comment_mock.assert_hits(1);
}

#[test]
fn merge_pull_request_sends_merge_method_and_parses_response() {
    let server = MockServer::start();
    let merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/octo/demo/pulls/42/merge")
            .query_param("access_token", "secret-token")
            .json_body(serde_json::json!({
                "merge_method": "squash"
            }));
        then.status(200).json_body(serde_json::json!({
            "sha": "abc123",
            "merged": true,
            "message": "Pull request merged"
        }));
    });

    let merge = client_for(&server)
        .merge_pull_request(
            "octo",
            "demo",
            42,
            "secret-token",
            &MergePullRequest {
                merge_method: "squash",
            },
        )
        .expect("merge response should parse");

    assert!(merge.merged);
    assert_eq!(merge.sha.as_deref(), Some("abc123"));
    merge_mock.assert_hits(1);
}

#[test]
fn merge_pull_request_preserves_api_error_description() {
    let server = MockServer::start();
    let merge_mock = server.mock(|when, then| {
        when.method(PUT).path("/v5/repos/octo/demo/pulls/42/merge");
        then.status(409).json_body(serde_json::json!({
            "error_description": "pull request has conflicts"
        }));
    });

    let result = client_for(&server).merge_pull_request(
        "octo",
        "demo",
        42,
        "secret-token",
        &MergePullRequest {
            merge_method: "merge",
        },
    );

    match result {
        Err(PullRequestError::UnexpectedStatusWithMessage(409, message)) => {
            assert_eq!(message, "pull request has conflicts");
        }
        _ => panic!("expected 409 API message"),
    }
    merge_mock.assert_hits(1);
}
