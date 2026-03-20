use std::path::Path;

use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn auth_status_reports_unauthenticated_when_no_token_is_available() {
    let config_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["authenticated"], false);
    assert_eq!(body["source"], "none");
    assert_eq!(body["username"], Value::Null);
    assert!(
        !config_file_exists(config_dir.path()),
        "status should not create a config file"
    );
}

#[test]
fn auth_login_persists_the_validated_token_for_later_status_checks() {
    let config_dir = TempDir::new().unwrap();
    let server = MockServer::start();

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "valid-token");
        then.status(200).json_body(serde_json::json!({
            "login": "octocat"
        }));
    });

    let login_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "login", "--token", "valid-token", "--json"])
        .output()
        .unwrap();

    assert_eq!(login_output.status.code(), Some(0));
    let login_body: Value = serde_json::from_slice(&login_output.stdout).unwrap();
    assert_eq!(login_body["authenticated"], true);
    assert_eq!(login_body["source"], "config");
    assert_eq!(login_body["username"], "octocat");

    let status_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(status_output.status.code(), Some(0));
    let status_body: Value = serde_json::from_slice(&status_output.stdout).unwrap();
    assert_eq!(status_body["authenticated"], true);
    assert_eq!(status_body["source"], "config");
    assert_eq!(status_body["username"], "octocat");
    user_mock.assert_hits(2);
}

#[test]
fn auth_login_can_read_the_token_from_stdin() {
    let config_dir = TempDir::new().unwrap();
    let server = MockServer::start();

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "stdin-token");
        then.status(200).json_body(serde_json::json!({
            "login": "stdin-user"
        }));
    });

    let login_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .write_stdin("stdin-token\n")
        .args(["auth", "login", "--with-token", "--json"])
        .output()
        .unwrap();

    assert_eq!(login_output.status.code(), Some(0));
    let login_body: Value = serde_json::from_slice(&login_output.stdout).unwrap();
    assert_eq!(login_body["authenticated"], true);
    assert_eq!(login_body["source"], "config");
    assert_eq!(login_body["username"], "stdin-user");

    let status_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(status_output.status.code(), Some(0));
    let status_body: Value = serde_json::from_slice(&status_output.stdout).unwrap();
    assert_eq!(status_body["authenticated"], true);
    assert_eq!(status_body["source"], "config");
    assert_eq!(status_body["username"], "stdin-user");
    user_mock.assert_hits(2);
}

#[test]
fn auth_status_prefers_the_environment_token_over_the_saved_config_token() {
    let config_dir = TempDir::new().unwrap();
    let server = MockServer::start();

    let config_token_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "config-token");
        then.status(200).json_body(serde_json::json!({
            "login": "config-user"
        }));
    });

    let env_token_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "env-token");
        then.status(200).json_body(serde_json::json!({
            "login": "env-user"
        }));
    });

    let login_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "login", "--token", "config-token", "--json"])
        .output()
        .unwrap();

    assert_eq!(login_output.status.code(), Some(0));

    let status_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "env-token")
        .args(["auth", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(status_output.status.code(), Some(0));
    let status_body: Value = serde_json::from_slice(&status_output.stdout).unwrap();
    assert_eq!(status_body["authenticated"], true);
    assert_eq!(status_body["source"], "env");
    assert_eq!(status_body["username"], "env-user");
    config_token_mock.assert_hits(1);
    env_token_mock.assert_hits(1);
}

#[test]
fn auth_logout_clears_the_saved_token_and_restores_unauthenticated_status() {
    let config_dir = TempDir::new().unwrap();
    let server = MockServer::start();

    let user_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user")
            .query_param("access_token", "config-token");
        then.status(200).json_body(serde_json::json!({
            "login": "config-user"
        }));
    });

    let login_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "login", "--token", "config-token", "--json"])
        .output()
        .unwrap();

    assert_eq!(login_output.status.code(), Some(0));

    let logout_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "logout", "--json"])
        .output()
        .unwrap();

    assert_eq!(logout_output.status.code(), Some(0));
    let logout_body: Value = serde_json::from_slice(&logout_output.stdout).unwrap();
    assert_eq!(logout_body["authenticated"], false);
    assert_eq!(logout_body["source"], "none");
    assert_eq!(logout_body["username"], Value::Null);
    assert_eq!(logout_body["logged_out"], true);

    let status_output = Command::cargo_bin("gitee-cli")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env_remove("GITEE_TOKEN")
        .args(["auth", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(status_output.status.code(), Some(3));
    let status_body: Value = serde_json::from_slice(&status_output.stdout).unwrap();
    assert_eq!(status_body["authenticated"], false);
    assert_eq!(status_body["source"], "none");
    assert_eq!(status_body["username"], Value::Null);
    user_mock.assert_hits(1);
}

fn config_file_exists(config_dir: &Path) -> bool {
    config_dir.join("config.toml").exists()
}
