use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;

#[test]
fn pr_list_supports_filters_in_json_output() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("state", "open")
            .query_param("author", "octocat")
            .query_param("assignee", "reviewer")
            .query_param("base", "main")
            .query_param("head", "feature/pr-list")
            .query_param("per_page", "2");
        then.status(200).json_body(serde_json::json!([
            {
                "number": 42,
                "state": "open",
                "title": "First PR",
                "body": "First body",
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
                    "ref": "feature/pr-list",
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
            },
            {
                "number": 43,
                "state": "open",
                "title": "Second PR",
                "body": "Second body",
                "html_url": "https://gitee.com/octo/demo/pulls/43",
                "draft": true,
                "mergeable": null,
                "created_at": "2026-03-20T11:00:00+08:00",
                "updated_at": "2026-03-20T12:00:00+08:00",
                "merged_at": null,
                "user": {
                    "login": "octocat"
                },
                "head": {
                    "ref": "feature/pr-list",
                    "sha": "abc124",
                    "repo": {
                        "full_name": "octo/demo"
                    }
                },
                "base": {
                    "ref": "main",
                    "sha": "def457",
                    "repo": {
                        "full_name": "octo/demo"
                    }
                }
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "pr",
            "list",
            "--repo",
            "octo/demo",
            "--state",
            "open",
            "--author",
            "octocat",
            "--assignee",
            "reviewer",
            "--base",
            "main",
            "--head",
            "feature/pr-list",
            "--limit",
            "2",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["source"], "explicit");
    assert_eq!(body["count"], 2);
    assert_eq!(body["pull_requests"][0]["number"], 42);
    assert_eq!(body["pull_requests"][0]["author"], "octocat");
    assert_eq!(body["pull_requests"][0]["head_ref"], "feature/pr-list");
    assert_eq!(body["pull_requests"][0]["base_ref"], "main");
    assert_eq!(body["pull_requests"][1]["number"], 43);
    assert_eq!(body["pull_requests"][1]["draft"], true);
    assert_eq!(body["pull_requests"][1]["mergeable"], Value::Null);

    pr_mock.assert_hits(1);
}

#[test]
fn pr_list_rejects_json_field_selection_until_list_projection_is_implemented() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "pr",
            "list",
            "--repo",
            "octo/demo",
            "--json",
            "number,title,url",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "pr list does not support selecting JSON fields yet"
    );
}

#[test]
fn pr_list_supports_default_text_output() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls")
            .query_param("per_page", "30");
        then.status(200).json_body(serde_json::json!([
            {
                "number": 42,
                "state": "open",
                "title": "First PR",
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
                    "ref": "feature/pr-list",
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
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "list", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "#42 open First PR (octocat)"
    );

    pr_mock.assert_hits(1);
}

#[test]
fn pr_list_fails_when_repository_is_missing() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/missing/pulls")
            .query_param("per_page", "30");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["pr", "list", "--repo", "octo/missing"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "repository not found"
    );

    pr_mock.assert_hits(1);
}
