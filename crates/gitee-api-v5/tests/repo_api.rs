mod common;

use common::{client_for, excludes_access_token};
use gitee_api_v5::RepoError;
use httpmock::Method::GET;
use httpmock::MockServer;

#[test]
fn fetch_repository_sends_optional_token_and_parses_repository() {
    let server = MockServer::start();
    let repo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo")
            .header("authorization", "Bearer secret-token")
            .matches(excludes_access_token);
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "human_name": "octo/示例",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let repo = client_for(&server)
        .fetch_repository("octo", "demo", Some("secret-token"))
        .expect("repository response should parse");

    assert_eq!(repo.full_name, "octo/demo");
    assert_eq!(repo.human_name.as_deref(), Some("octo/示例"));
    assert_eq!(repo.path, "demo");
    assert_eq!(repo.default_branch, "main");
    assert!(!repo.fork);
    repo_mock.assert_hits(1);
}

#[test]
fn fetch_repository_maps_not_found() {
    let server = MockServer::start();
    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/missing");
        then.status(404).json_body(serde_json::json!({
            "message": "not found"
        }));
    });

    let result = client_for(&server).fetch_repository("octo", "missing", None);

    assert!(matches!(result, Err(RepoError::NotFound)));
    repo_mock.assert_hits(1);
}

#[test]
fn find_repository_by_human_name_matches_slug_or_human_name() {
    let server = MockServer::start();
    let repos_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user/repos")
            .header("authorization", "Bearer secret-token")
            .matches(excludes_access_token);
        then.status(200).json_body(serde_json::json!([
            {
                "full_name": "octo/other",
                "human_name": "octo/其它",
                "path": "other",
                "fork": false,
                "default_branch": "main"
            },
            {
                "full_name": "octo/demo-slug",
                "human_name": "octo/demo",
                "path": "demo-slug",
                "fork": false,
                "default_branch": "master"
            }
        ]));
    });

    let repo = client_for(&server)
        .find_repository_by_human_name("octo", "demo", "secret-token")
        .expect("repository list should parse")
        .expect("human name should match");

    assert_eq!(repo.full_name, "octo/demo-slug");
    assert_eq!(repo.human_name.as_deref(), Some("octo/demo"));
    repos_mock.assert_hits(1);
}
