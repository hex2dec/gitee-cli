use assert_cmd::Command;
use httpmock::Method::{GET, PUT};
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_merge_uses_default_merge_strategy_in_json_output() {
    let server = MockServer::start();

    let merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/octo/demo/pulls/42/merge")
            .query_param("access_token", "secret-token")
            .header("content-type", "application/json")
            .body_contains("\"merge_method\":\"merge\"");
        then.status(200).json_body(serde_json::json!({
            "sha": "abc123",
            "merged": true,
            "message": "Pull Request merged"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "merge", "42", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["pull_request"], 42);
    assert_eq!(body["merge_method"], "merge");
    assert_eq!(body["merged"], true);
    assert_eq!(body["sha"], "abc123");
    assert_eq!(body["message"], "Pull Request merged");

    merge_mock.assert_hits(1);
}

#[test]
fn pr_merge_supports_squash_strategy_with_local_repo_context() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/merge");

    let merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/octo/demo/pulls/43/merge")
            .query_param("access_token", "secret-token")
            .header("content-type", "application/json")
            .body_contains("\"merge_method\":\"squash\"");
        then.status(200).json_body(serde_json::json!({
            "sha": "def456",
            "merged": true,
            "message": "Squash merged"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "merge", "43", "--squash"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Merged pull request #43
repository: octo/demo
merge_method: squash
sha: def456
message: Squash merged"
    );

    merge_mock.assert_hits(1);
}

#[test]
fn pr_merge_resolves_human_name_remote_to_canonical_private_repo() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:hzw/tip-ucan.git", "feature/merge");

    let direct_merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/hzw/tip-ucan/pulls/44/merge")
            .query_param("access_token", "secret-token")
            .body_contains("\"merge_method\":\"rebase\"");
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

    let canonical_merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/hzw-dev/tip-ucan/pulls/44/merge")
            .query_param("access_token", "secret-token")
            .body_contains("\"merge_method\":\"rebase\"");
        then.status(200).json_body(serde_json::json!({
            "sha": "fedcba",
            "merged": true,
            "message": "Rebased and merged"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "merge", "44", "--rebase", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["merge_method"], "rebase");
    assert_eq!(body["sha"], "fedcba");

    direct_merge_mock.assert_hits(1);
    repo_list_mock.assert_hits(1);
    canonical_merge_mock.assert_hits(1);
}

#[test]
fn pr_merge_requires_authentication() {
    let config_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args(["pr", "merge", "42", "--repo", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr merge"
    );
}

#[test]
fn pr_merge_rejects_conflicting_strategy_flags() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args([
            "pr",
            "merge",
            "42",
            "--repo",
            "octo/demo",
            "--merge",
            "--squash",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "provide only one of --merge, --squash, or --rebase"
    );
}

#[test]
fn pr_merge_surfaces_remote_validation_errors() {
    let server = MockServer::start();

    let merge_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/v5/repos/octo/demo/pulls/45/merge")
            .query_param("access_token", "secret-token")
            .body_contains("\"merge_method\":\"merge\"");
        then.status(405).json_body(serde_json::json!({
            "message": "pull request is not mergeable"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "merge", "45", "--repo", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "remote request failed (405): pull request is not mergeable"
    );

    merge_mock.assert_hits(1);
}

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
    let repo_dir = TempDir::new().unwrap();

    run_git(repo_dir.path(), &["init"]);
    run_git(repo_dir.path(), &["checkout", "-b", branch]);
    run_git(repo_dir.path(), &["remote", "add", "origin", remote_url]);

    repo_dir
}

fn run_git(repo_dir: &Path, args: &[&str]) {
    let output = ProcessCommand::new("git")
        .args(args)
        .current_dir(repo_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git command failed: git {}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
