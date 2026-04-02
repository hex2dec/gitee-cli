use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

#[test]
fn repo_clone_clones_to_explicit_destination_over_https_and_reports_json() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("explicit-dest");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "/definitely/missing/ssh-demo.git",
            "clone_url": remote_repo.path(),
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "repo",
            "clone",
            "octo/demo",
            destination.file_name().unwrap().to_str().unwrap(),
            "--https",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cloning into"));

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["full_name"], "octo/demo");
    assert_eq!(body["transport"], "https");
    assert_eq!(
        body["destination"],
        destination.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(body["clone_url"], remote_repo.path().display().to_string());
    assert!(destination.join(".git").exists());
    assert_eq!(
        std::fs::read_to_string(destination.join("README.md")).unwrap(),
        "hello from remote\n"
    );

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_uses_ssh_transport_and_defaults_destination_to_repo_name() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("demo");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": remote_repo.path().display().to_string(),
            "clone_url": "/definitely/missing/https-demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "clone", "octo/demo", "--ssh"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cloning into"));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!(
            "Cloned octo/demo to {} via ssh",
            destination.canonicalize().unwrap().display()
        )
    );
    assert!(destination.join(".git").exists());
    assert_eq!(
        std::fs::read_to_string(destination.join("README.md")).unwrap(),
        "hello from remote\n"
    );

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_uses_saved_protocol_preference_when_no_transport_flag_is_provided() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("demo");

    write_config(
        config_dir.path(),
        r#"
clone_protocol = "ssh"
"#,
    );

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": remote_repo.path().display().to_string(),
            "clone_url": "/definitely/missing/https-demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .args(["repo", "clone", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["full_name"], "octo/demo");
    assert_eq!(body["transport"], "ssh");
    assert_eq!(
        body["destination"],
        destination.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(body["clone_url"], remote_repo.path().display().to_string());
    assert!(destination.join(".git").exists());

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_prompts_for_protocol_on_first_use_and_persists_the_choice() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("demo");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": remote_repo.path().display().to_string(),
            "clone_url": "/definitely/missing/https-demo.git",
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .write_stdin("ssh\n")
        .args(["repo", "clone", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["transport"], "ssh");
    assert_eq!(
        body["destination"],
        destination.canonicalize().unwrap().display().to_string()
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No saved clone protocol preference."));
    assert!(stderr.contains("Choose clone protocol [ssh/https]: "));

    assert_eq!(
        std::fs::read_to_string(config_dir.path().join("config.toml")).unwrap(),
        "clone_protocol = \"ssh\"\n"
    );

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_does_not_silently_default_protocol_when_the_prompt_is_unanswered() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("demo");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": remote_repo.path().display().to_string(),
            "clone_url": remote_repo.path().display().to_string(),
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .args(["repo", "clone", "octo/demo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No saved clone protocol preference."));
    assert!(stderr.contains("Choose clone protocol [ssh/https]: "));
    assert!(stderr.contains("clone protocol must be selected as ssh or https"));

    assert!(!destination.exists());
    assert!(!config_dir.path().join("config.toml").exists());

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_streams_git_progress_to_stderr_without_corrupting_json_output() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("progress-dest");

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "/definitely/missing/ssh-demo.git",
            "clone_url": file_url(remote_repo.path()),
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args([
            "repo",
            "clone",
            "octo/demo",
            destination.file_name().unwrap().to_str().unwrap(),
            "--https",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["transport"], "https");
    assert_eq!(
        body["destination"],
        destination.canonicalize().unwrap().display().to_string()
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cloning into"));
    assert!(stderr.contains("Receiving objects:"));

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_persists_first_use_protocol_choice_without_overwriting_saved_token() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();

    write_config(
        config_dir.path(),
        r#"
token = "saved-token"
"#,
    );

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "/definitely/missing/ssh-demo.git",
            "clone_url": remote_repo.path().display().to_string(),
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .env("GITEE_CONFIG_DIR", config_dir.path())
        .write_stdin("https\n")
        .args(["repo", "clone", "octo/demo", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["transport"], "https");

    let config = std::fs::read_to_string(config_dir.path().join("config.toml")).unwrap();
    assert!(config.contains("token = \"saved-token\""));
    assert!(config.contains("clone_protocol = \"https\""));

    repo_mock.assert_hits(1);
}

#[test]
fn repo_clone_rejects_an_invalid_repository_slug() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["repo", "clone", "octo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "invalid repository slug: expected owner/repo"
    );
}

#[test]
fn repo_clone_fails_with_a_stable_git_error_when_destination_conflicts() {
    let server = MockServer::start();
    let remote_repo = seeded_bare_repository();
    let working_dir = TempDir::new().unwrap();
    let destination = working_dir.path().join("occupied");

    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("README.md"), "already here\n").unwrap();

    let repo_mock = server.mock(|when, then| {
        when.method(GET).path("/v5/repos/octo/demo");
        then.status(200).json_body(serde_json::json!({
            "full_name": "octo/demo",
            "path": "demo",
            "html_url": "https://gitee.com/octo/demo",
            "ssh_url": "/definitely/missing/ssh-demo.git",
            "clone_url": remote_repo.path().display().to_string(),
            "fork": false,
            "default_branch": "main"
        }));
    });

    let output = Command::cargo_bin("gitee")
        .unwrap()
        .current_dir(working_dir.path())
        .env("GITEE_BASE_URL", server.base_url())
        .args(["repo", "clone", "octo/demo", "occupied", "--https"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "clone destination already exists: occupied"
    );
    assert_eq!(
        std::fs::read_to_string(destination.join("README.md")).unwrap(),
        "already here\n"
    );

    repo_mock.assert_hits(1);
}

fn seeded_bare_repository() -> TempDir {
    let bare_repo = TempDir::new().unwrap();
    let source_repo = TempDir::new().unwrap();

    run_git(bare_repo.path(), &["init", "--bare"]);
    run_git(source_repo.path(), &["init"]);
    std::fs::write(source_repo.path().join("README.md"), "hello from remote\n").unwrap();
    run_git(source_repo.path(), &["add", "README.md"]);
    run_git(
        source_repo.path(),
        &[
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "seed",
        ],
    );
    run_git(source_repo.path(), &["branch", "-M", "main"]);
    run_git(
        source_repo.path(),
        &[
            "remote",
            "add",
            "origin",
            bare_repo.path().to_str().unwrap(),
        ],
    );
    run_git(source_repo.path(), &["push", "-u", "origin", "main"]);
    run_git(
        bare_repo.path(),
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    );

    bare_repo
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

fn write_config(config_dir: &Path, contents: &str) {
    std::fs::write(config_dir.join("config.toml"), contents.trim_start()).unwrap();
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}
