use gitee_api_v5::GiteeClient;

#[test]
fn client_defaults_to_gitee_v5_api_base() {
    let client = GiteeClient::new(None);

    assert_eq!(client.base_url(), "https://gitee.com/api");
}

#[test]
fn client_trims_trailing_slash_from_custom_base_url() {
    let client = GiteeClient::new(Some("http://127.0.0.1:1234/"));

    assert_eq!(client.base_url(), "http://127.0.0.1:1234");
}
