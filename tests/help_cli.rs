use assert_cmd::Command;
use serde_json::Value;

#[test]
fn root_help_describes_the_cli_and_agent_discovery_entrypoint() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["--help"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Agent-first CLI for gitee.com"));
    assert!(stdout.contains("Authenticate with gitee.com and inspect login state"));
    assert!(stdout.contains("Use `gitee help --json`"));
}

#[test]
fn help_json_exposes_the_top_level_command_groups() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["schema_version"], 1);
    assert_eq!(
        body["agent_guidance"]["recommended_discovery_command"],
        "gitee help --json"
    );

    let commands = body["commands"].as_array().unwrap();
    let names = commands
        .iter()
        .map(|command| command["name"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["auth", "issue", "pr", "repo"]);

    let pr_group = commands
        .iter()
        .find(|command| command["name"] == "pr")
        .unwrap();
    assert_eq!(pr_group["gh_equivalent"], "gh pr");
}

#[test]
fn help_can_render_text_for_a_nested_command_topic() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "create"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Create a pull request from the current branch or an explicit head"));
    assert!(stdout.contains("Usage: gitee pr create [OPTIONS]"));
    assert!(stdout.contains("--title <TITLE>"));
}

#[test]
fn help_json_can_describe_a_single_nested_command() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "create", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["path"], "pr create");
    assert_eq!(body["gh_equivalent"], "gh pr create");
    assert_eq!(body["auth"], "required");
    assert_eq!(body["repo_inference"], true);

    let flags = body["flags"].as_array().unwrap();
    assert!(flags.iter().any(|flag| flag["name"] == "--title"));
    assert!(flags.iter().any(|flag| flag["name"] == "--body-file"));

    let input_sources = body["input_sources"].as_array().unwrap();
    assert!(input_sources.iter().any(|source| source == "--body"));
    assert!(input_sources.iter().any(|source| source == "--body-file"));
}

#[test]
fn help_json_can_describe_pr_edit() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "edit", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["path"], "pr edit");
    assert_eq!(body["gh_equivalent"], "gh pr edit");
    assert_eq!(body["auth"], "required");
    assert_eq!(body["repo_inference"], true);

    let flags = body["flags"].as_array().unwrap();
    assert!(flags.iter().any(|flag| flag["name"] == "--title"));
    assert!(flags.iter().any(|flag| flag["name"] == "--state"));
    assert!(flags.iter().any(|flag| flag["name"] == "--draft"));
    assert!(flags.iter().any(|flag| flag["name"] == "--ready"));
}
