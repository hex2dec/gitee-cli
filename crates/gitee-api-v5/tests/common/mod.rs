use gitee_api_v5::GiteeClient;
use httpmock::MockServer;

pub fn client_for(server: &MockServer) -> GiteeClient {
    GiteeClient::new(Some(&server.base_url()))
}
