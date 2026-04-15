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
fn help_text_describes_json_usage_per_command() {
    let pr_list_output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "list"])
        .output()
        .unwrap();

    assert_eq!(pr_list_output.status.code(), Some(0));
    assert!(pr_list_output.stderr.is_empty());
    let pr_list_stdout = String::from_utf8_lossy(&pr_list_output.stdout);
    assert!(pr_list_stdout.contains("--json [<FIELDS>]"));

    for topic in [
        ["help", "issue", "create"],
        ["help", "issue", "comment"],
        ["help", "pr", "comment"],
        ["help", "pr", "checkout"],
        ["help", "repo", "clone"],
    ] {
        let output = Command::cargo_bin("gitee")
            .unwrap()
            .args(topic)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--json"));
        assert!(!stdout.contains("--json [<FIELDS>]"));
    }
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

#[test]
fn help_json_describes_json_field_selection_for_list_and_status_commands() {
    let pr_list_output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "list", "--json"])
        .output()
        .unwrap();

    assert_eq!(pr_list_output.status.code(), Some(0));
    assert!(pr_list_output.stderr.is_empty());

    let pr_list_body: Value = serde_json::from_slice(&pr_list_output.stdout).unwrap();
    assert_eq!(pr_list_body["json_field_selection"], true);
    let pr_list_fields = pr_list_body["json_fields"].as_array().unwrap();
    assert!(pr_list_fields.iter().any(|field| field == "number"));
    assert!(pr_list_fields.iter().any(|field| field == "title"));
    assert!(pr_list_fields.iter().any(|field| field == "url"));
    assert!(pr_list_fields.iter().any(|field| field == "state"));
    assert!(pr_list_fields.iter().any(|field| field == "createdAt"));
    assert!(pr_list_fields.iter().any(|field| field == "isDraft"));
    let pr_list_examples = pr_list_body["examples"].as_array().unwrap();
    assert!(
        pr_list_examples.iter().any(|example| example
            == "gitee pr list --repo octo/demo --limit 10 --json number,title,url")
    );

    let pr_status_output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "pr", "status", "--json"])
        .output()
        .unwrap();

    assert_eq!(pr_status_output.status.code(), Some(0));
    assert!(pr_status_output.stderr.is_empty());

    let pr_status_body: Value = serde_json::from_slice(&pr_status_output.stdout).unwrap();
    assert_eq!(pr_status_body["json_field_selection"], true);
    let pr_status_fields = pr_status_body["json_fields"].as_array().unwrap();
    assert!(pr_status_fields.iter().any(|field| field == "number"));
    assert!(pr_status_fields.iter().any(|field| field == "title"));
    assert!(pr_status_fields.iter().any(|field| field == "url"));
    assert!(pr_status_fields.iter().any(|field| field == "state"));
    assert!(pr_status_fields.iter().any(|field| field == "createdAt"));
    assert!(pr_status_fields.iter().any(|field| field == "isDraft"));
    let pr_status_examples = pr_status_body["examples"].as_array().unwrap();
    assert!(
        pr_status_examples
            .iter()
            .any(|example| example == "gitee pr status --json number,title,url")
    );
}
