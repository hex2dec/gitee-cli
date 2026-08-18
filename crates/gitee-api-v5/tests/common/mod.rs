use gitee_api_v5::GiteeClient;
use httpmock::MockServer;
use httpmock::prelude::HttpMockRequest;

pub fn client_for(server: &MockServer) -> GiteeClient {
    GiteeClient::new(Some(&server.base_url()))
}

pub fn excludes_access_token(request: &HttpMockRequest) -> bool {
    let query_contains_token = request.query_params.as_ref().is_some_and(|query_params| {
        query_params
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("access_token"))
    });

    let body_contains_token = request.body.as_ref().is_some_and(|body| {
        String::from_utf8_lossy(body)
            .to_ascii_lowercase()
            .contains("access_token")
    });

    !query_contains_token && !body_contains_token
}
