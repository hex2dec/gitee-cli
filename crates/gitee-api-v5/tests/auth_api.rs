mod common;

use common::{client_for, excludes_access_token};
use gitee_api_v5::AuthError;
use httpmock::Method::GET;
use httpmock::MockServer;

#[test]
fn fetch_current_user_sends_token_and_returns_login() {
    let server = MockServer::start();
    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .header("authorization", "Bearer secret-token")
            .matches(excludes_access_token);
        then.status(200).json_body(serde_json::json!({
            "login": "octocat"
        }));
    });

    let login = client_for(&server)
        .fetch_current_user("secret-token")
        .expect("user response should parse");

    assert_eq!(login, "octocat");
    user_mock.assert_hits(1);
}

#[test]
fn fetch_current_user_maps_unauthorized_to_invalid_token() {
    let server = MockServer::start();
    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .header("authorization", "Bearer bad-token")
            .matches(excludes_access_token);
        then.status(401).json_body(serde_json::json!({
            "message": "invalid token"
        }));
    });

    let result = client_for(&server).fetch_current_user("bad-token");

    assert!(matches!(result, Err(AuthError::InvalidToken)));
    user_mock.assert_hits(1);
}
