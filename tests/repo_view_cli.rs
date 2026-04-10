use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn repo_view_supports_explicit_repo_slug_in_json_output() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "name": "demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "view", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "explicit");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["full_name"], "octo/demo");
    assert_eq!(body["default_branch"], "main");
    assert_eq!(body["current_branch"], Value::Null);
    assert_eq!(body["html_url"], "https://gitee.com/octo/demo");
    assert_eq!(body["ssh_url"], "git@gitee.com:octo/demo.git");
    assert_eq!(body["clone_url"], "https://gitee.com/octo/demo.git");
    assert_eq!(body["fork"], false);

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_supports_gh_style_json_field_selection() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "name": "demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "repo",
            "view",
            "--repo",
            "octo/demo",
            "--json",
            "nameWithOwner,url",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["nameWithOwner"], "octo/demo");
    assert_eq!(body["url"], "https://gitee.com/octo/demo");

    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 2);
    assert!(!object.contains_key("full_name"));
    assert!(!object.contains_key("html_url"));

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_handles_private_repo_payload_without_clone_url() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/hzw-dev/tip-ucan");
        then.status(200).json_body(serde_json::json!({
            "full_name": "hzw-dev/tip-ucan",
            "human_name": "hzw/tip-ucan",
            "path": "tip-ucan",
            "html_url": "https://gitee.com/hzw-dev/tip-ucan.git",
            "ssh_url": "git@gitee.com:hzw-dev/tip-ucan.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "view", "--repo", "hzw-dev/tip-ucan", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["full_name"], "hzw-dev/tip-ucan");
    assert_eq!(body["owner"], "hzw-dev");
    assert_eq!(body["name"], "tip-ucan");
    assert_eq!(body["html_url"], "https://gitee.com/hzw-dev/tip-ucan");
    assert_eq!(body["clone_url"], "https://gitee.com/hzw-dev/tip-ucan.git");
    assert_eq!(body["ssh_url"], "git@gitee.com:hzw-dev/tip-ucan.git");

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_infers_repository_and_current_branch_from_https_origin() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/https");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "name": "demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "view", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "local");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["full_name"], "octo/demo");
    assert_eq!(body["default_branch"], "main");
    assert_eq!(body["current_branch"], "feature/https");

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_infers_repository_from_ssh_origin() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:octo/demo.git", "feature/ssh");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "name": "demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "view", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "local");
    assert_eq!(body["owner"], "octo");
    assert_eq!(body["name"], "demo");
    assert_eq!(body["current_branch"], "feature/ssh");

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_resolves_human_name_remote_to_canonical_private_repo() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:hzw/tip-ucan.git", "feature/human-name");

    let repo_lookup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/hzw/tip-ucan")
            .query_param("access_token", "secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_list_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user/repos")
            .query_param("access_token", "secret-token");
        then.status(200).json_body(serde_json::json!([
            {
                "full_name": "hzw-dev/tip-ucan",
                "human_name": "hzw/tip-ucan",
                "path": "tip-ucan",
                "html_url": "https://gitee.com/hzw-dev/tip-ucan.git",
                "ssh_url": "git@gitee.com:hzw-dev/tip-ucan.git",
                "fork": false,
                "default_branch": "main"
            }
        ]));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["repo", "view", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["source"], "local");
    assert_eq!(body["owner"], "hzw-dev");
    assert_eq!(body["name"], "tip-ucan");
    assert_eq!(body["full_name"], "hzw-dev/tip-ucan");
    assert_eq!(body["current_branch"], "feature/human-name");

    repo_lookup_mock.assert_hits(1);
    repo_list_mock.assert_hits(1);
}

#[test]
fn repo_view_renders_stable_text_output() {
    let server = MockServer::start();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "name": "demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "git@gitee.com:octo/demo.git",
            "clone_url": "https://gitee.com/octo/demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "view", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
octo/demo
default branch: main
current branch: (none)
fork: false
html url: https://gitee.com/octo/demo
ssh url: git@gitee.com:octo/demo.git
clone url: https://gitee.com/octo/demo.git
source: explicit"
    );
    assert!(output.stderr.is_empty());

    repo_mock.assert_hits(1);
}

#[test]
fn repo_view_fails_with_a_stable_git_error_when_head_is_detached() {
    let repo_dir = git_repo_with_detached_head("https://gitee.com/octo/demo.git");

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .args(["repo", "view", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "git context error: HEAD is detached"
    );
}

#[test]
fn repo_view_fails_when_not_inside_a_git_repository() {
    let working_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .args(["repo", "view", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "git context error: not inside a git repository"
    );
}

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
    let repo_dir = TempDir::new().unwrap();

    run_git(repo_dir.path(), &["init"]);
    run_git(repo_dir.path(), &["checkout", "-b", branch]);
    run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);

    repo_dir
}

fn git_repo_with_detached_head(remote_url: &str) -> TempDir {
    let repo_dir = TempDir::new().unwrap();

    run_git(repo_dir.path(), &["init"]);
    std::fs::write(repo_dir.path().join("README.md"), "hello\n").unwrap();
    run_git(repo_dir.path(), &["add", "README.md"]);
    run_git(
        repo_dir.path(),
        &[
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "init",
        ],
    );
    run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);
    run_git(repo_dir.path(), &["checkout", "--detach"]);

    repo_dir
}

fn run_git(repo_dir: &Path, args: &[&str]) {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(repo_dir)
        .status()
        .unwrap();

    assert!(
        status.success(),
        "git command failed: git {}",
        args.join(" ")
    );
}
