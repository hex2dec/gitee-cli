use assert_cmd::Command;
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn pr_review_approves_from_local_repo_context_in_json_output() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("https://gitee.com/octo/demo.git", "feature/review");

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/42")
            .header("authorization", "Bearer secret-token");
        then.status(200).json_body(pull_request_payload(42));
    });

    let review_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/42/review")
            .header("authorization", "Bearer secret-token");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "review", "42", "--approve", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["action"], "approve");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["pull_request"], 42);
    assert_eq!(body.as_object().unwrap().len(), 3);

    pr_mock.assert_hits(1);
    review_mock.assert_hits(1);
}

#[test]
fn pr_review_posts_a_comment_in_json_output() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/43")
            .header("authorization", "Bearer secret-token");
        then.status(200).json_body(pull_request_payload(43));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/43/comments")
            .header("authorization", "Bearer secret-token")
            .body_contains("body=Looks+good");
        then.status(201).json_body(serde_json::json!({
            "id": 101,
            "body": "Looks good",
            "html_url": "https://gitee.com/octo/demo/pulls/43#note_101",
            "comment_type": "pr_comment",
            "created_at": "2026-07-10T09:00:00+08:00",
            "updated_at": "2026-07-10T09:00:00+08:00",
            "user": {
                "login": "reviewer"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "review",
            "43",
            "--repo",
            "octo/demo",
            "--comment",
            "--body",
            "Looks good",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["action"], "comment");
    assert_eq!(body["id"], 101);
    assert_eq!(body["body"], "Looks good");
    assert_eq!(body["author"], "reviewer");
    assert_eq!(body["repository"], "octo/demo");
    assert_eq!(body["pull_request"], 43);
    assert_eq!(body["comment_type"], "pr_comment");

    pr_mock.assert_hits(1);
    comment_mock.assert_hits(1);
}

#[test]
fn pr_review_reads_comment_body_from_stdin_with_short_flags() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/44")
            .header("authorization", "Bearer secret-token");
        then.status(200).json_body(pull_request_payload(44));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/44/comments")
            .header("authorization", "Bearer secret-token")
            .body_contains("body=Generated+review%0A");
        then.status(201).json_body(serde_json::json!({
            "id": 102,
            "body": "Generated review\n",
            "html_url": "https://gitee.com/octo/demo/pulls/44#note_102",
            "comment_type": "pr_comment",
            "created_at": "2026-07-10T10:00:00+08:00",
            "updated_at": "2026-07-10T10:00:00+08:00",
            "user": {
                "login": "reviewer"
            }
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "review", "44", "--repo", "octo/demo", "-c", "-F", "-"])
        .write_stdin("Generated review\n")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "\
Reviewed pull request #44 with a comment
comment id: 102
repository: octo/demo
author: reviewer
url: https://gitee.com/octo/demo/pulls/44#note_102"
    );

    pr_mock.assert_hits(1);
    comment_mock.assert_hits(1);
}

#[test]
fn pr_review_rejects_an_empty_comment_before_remote_lookup() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo/pulls/45");
        then.status(200).json_body(pull_request_payload(45));
    });

    let comment_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/45/comments");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args([
            "pr",
            "review",
            "45",
            "--repo",
            "octo/demo",
            "--comment",
            "--body",
            "   ",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "comment body cannot be empty"
    );

    pr_mock.assert_hits(0);
    comment_mock.assert_hits(0);
}

#[test]
fn pr_review_resolves_human_name_remote_to_canonical_private_repo() {
    let server = MockServer::start();
    let repo_dir = git_repo_with_remote("git@gitee.com:hzw/tip-ucan.git", "feature/review");

    let direct_lookup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/hzw/tip-ucan/pulls/46")
            .header("authorization", "Bearer secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_list_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/user/repos")
            .header("authorization", "Bearer secret-token");
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

    let canonical_pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/hzw-dev/tip-ucan/pulls/46")
            .header("authorization", "Bearer secret-token");
        then.status(200)
            .json_body(pull_request_payload_for(46, "hzw-dev/tip-ucan"));
    });

    let review_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/hzw-dev/tip-ucan/pulls/46/review")
            .header("authorization", "Bearer secret-token");
        then.status(201);
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(repo_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "review", "46", "--approve", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["repository"], "hzw-dev/tip-ucan");
    assert_eq!(body["pull_request"], 46);

    direct_lookup_mock.assert_hits(1);
    repo_list_mock.assert_hits(1);
    canonical_pr_mock.assert_hits(1);
    review_mock.assert_hits(1);
}

#[test]
fn pr_review_requires_authentication() {
    let config_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .env_remove("GITEE_TOKEN")
        .args(["pr", "review", "42", "--repo", "octo/demo", "--approve"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "authentication required for pr review"
    );
}

#[test]
fn pr_review_reports_a_missing_pull_request() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/404")
            .header("authorization", "Bearer secret-token");
        then.status(404).json_body(serde_json::json!({
            "message": "Not Found"
        }));
    });

    let repo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo")
            .header("authorization", "Bearer secret-token");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
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
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "review", "404", "--repo", "octo/demo", "--approve"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "pull request not found"
    );

    pr_mock.assert_hits(1);
    repo_mock.assert_hits(1);
}

#[test]
fn pr_review_surfaces_remote_validation_errors() {
    let server = MockServer::start();

    let pr_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v5/repos/octo/demo/pulls/47")
            .header("authorization", "Bearer secret-token");
        then.status(200).json_body(pull_request_payload(47));
    });

    let review_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v5/repos/octo/demo/pulls/47/review")
            .header("authorization", "Bearer secret-token");
        then.status(400).json_body(serde_json::json!({
            "message": "pull request has already been reviewed"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_TOKEN", "secret-token")
        .args(["pr", "review", "47", "--repo", "octo/demo", "--approve"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "remote request failed (400): pull request has already been reviewed"
    );

    pr_mock.assert_hits(1);
    review_mock.assert_hits(1);
}

#[test]
fn pr_review_rejects_invalid_action_and_body_combinations() {
    let cases = [
        (
            vec!["pr", "review", "42", "--repo", "octo/demo"],
            "provide exactly one of --approve or --comment",
        ),
        (
            vec![
                "pr",
                "review",
                "42",
                "--repo",
                "octo/demo",
                "--approve",
                "--comment",
                "--body",
                "Needs work",
            ],
            "provide exactly one of --approve or --comment",
        ),
        (
            vec!["pr", "review", "42", "--repo", "octo/demo", "--comment"],
            "pr review --comment requires --body or --body-file",
        ),
        (
            vec![
                "pr",
                "review",
                "42",
                "--repo",
                "octo/demo",
                "--approve",
                "--body",
                "Looks good",
            ],
            "review body is not supported with --approve",
        ),
    ];

    for (args, expected_error) in cases {
        let output = Command::cargo_bin("gitee")
            .unwrap()
            .env("GITEE_TOKEN", "secret-token")
            .args(args)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            expected_error
        );
    }
}

fn pull_request_payload(number: u64) -> serde_json::Value {
    pull_request_payload_for(number, "octo/demo")
}

fn pull_request_payload_for(number: u64, repository: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "open",
        "title": "PR review target",
        "body": null,
        "html_url": format!("https://gitee.com/{repository}/pulls/{number}"),
        "draft": false,
        "mergeable": true,
        "created_at": "2026-07-10T08:00:00+08:00",
        "updated_at": "2026-07-10T08:30:00+08:00",
        "merged_at": null,
        "user": {
            "login": "octocat"
        },
        "head": {
            "ref": "feature/review",
            "sha": "abc123",
            "repo": {
                "full_name": repository
            }
        },
        "base": {
            "ref": "main",
            "sha": "def456",
            "repo": {
                "full_name": repository
            }
        }
    })
}

fn git_repo_with_remote(remote_url: &str, branch: &str) -> TempDir {
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
