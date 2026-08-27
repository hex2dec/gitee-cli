use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn skills_install_copies_the_bundled_skill_into_the_agents_skills_dir() {
    let home_dir = TempDir::new().unwrap();
    let skill_dir = skill_dir(home_dir.path());

    let output = gitee_with_home(home_dir.path())
        .args(["skills", "install"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("installed using-gitee-cli to {}", skill_dir.display())
    );

    assert!(skill_dir.join("SKILL.md").is_file());
    assert!(skill_dir.join("references/commands.md").is_file());

    let skill_body = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_body.contains("name: using-gitee-cli"));
    assert_eq!(read_tree(&source_skill_dir()), read_tree(&skill_dir));
}

#[test]
fn skills_install_updates_an_existing_installation_and_reports_json() {
    let home_dir = TempDir::new().unwrap();
    let skill_dir = skill_dir(home_dir.path());
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "stale").unwrap();

    let output = gitee_with_home(home_dir.path())
        .args(["skills", "install", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["name"], "using-gitee-cli");
    assert_eq!(body["agent"], "default");
    assert_eq!(body["installed"], true);
    assert_eq!(body["action"], "updated");
    assert_eq!(body["path"], skill_dir.display().to_string());

    let skill_body = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_body.contains("name: using-gitee-cli"));
    assert!(!skill_body.contains("stale"));
}

#[test]
fn skills_list_reports_each_target_and_supports_agent_filtering() {
    let home_dir = TempDir::new().unwrap();
    let default_dir = skill_dir(home_dir.path());
    let claude_dir = claude_code_skill_dir(home_dir.path());

    let initial = gitee_with_home(home_dir.path())
        .args(["skills", "list"])
        .output()
        .unwrap();

    assert_eq!(initial.status.code(), Some(0));
    assert!(initial.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&initial.stdout).trim(),
        format!(
            "using-gitee-cli (default)\tnot installed\t{}\nusing-gitee-cli claude-code\tnot installed\t{}",
            default_dir.display(),
            claude_dir.display()
        )
    );

    let initial_json = gitee_with_home(home_dir.path())
        .args(["skills", "list", "--json"])
        .output()
        .unwrap();

    assert_eq!(initial_json.status.code(), Some(0));
    assert!(initial_json.stderr.is_empty());
    let initial_body: Value = serde_json::from_slice(&initial_json.stdout).unwrap();
    let rows = initial_body.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "using-gitee-cli");
    assert_eq!(rows[0]["agent"], "default");
    assert_eq!(rows[0]["installed"], false);
    assert_eq!(rows[0]["path"], default_dir.display().to_string());
    assert_eq!(rows[1]["name"], "using-gitee-cli");
    assert_eq!(rows[1]["agent"], "claude-code");
    assert_eq!(rows[1]["installed"], false);
    assert_eq!(rows[1]["path"], claude_dir.display().to_string());

    // Install the default target, then confirm it is reflected and that
    // --agent filters to a single row.
    let install = gitee_with_home(home_dir.path())
        .args(["skills", "install"])
        .output()
        .unwrap();
    assert_eq!(install.status.code(), Some(0));

    let after_install = gitee_with_home(home_dir.path())
        .args(["skills", "list"])
        .output()
        .unwrap();
    assert_eq!(after_install.status.code(), Some(0));
    let after_stdout = String::from_utf8_lossy(&after_install.stdout);
    assert!(after_stdout.contains(&format!(
        "using-gitee-cli (default)\tinstalled\t{}",
        default_dir.display()
    )));
    assert!(after_stdout.contains(&format!(
        "using-gitee-cli claude-code\tnot installed\t{}",
        claude_dir.display()
    )));

    let filtered = gitee_with_home(home_dir.path())
        .args(["skills", "list", "--agent", "claude-code", "--json"])
        .output()
        .unwrap();

    assert_eq!(filtered.status.code(), Some(0));
    assert!(filtered.stderr.is_empty());
    let filtered_body: Value = serde_json::from_slice(&filtered.stdout).unwrap();
    let rows = filtered_body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["agent"], "claude-code");
    assert_eq!(rows[0]["installed"], false);
    assert_eq!(rows[0]["path"], claude_dir.display().to_string());

    let filtered_text = gitee_with_home(home_dir.path())
        .args(["skills", "ls", "--agent", "claude-code"])
        .output()
        .unwrap();

    assert_eq!(filtered_text.status.code(), Some(0));
    assert!(filtered_text.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&filtered_text.stdout).trim(),
        format!(
            "using-gitee-cli claude-code\tnot installed\t{}",
            claude_dir.display()
        )
    );
}

#[test]
fn skills_uninstall_removes_the_exact_skill_directory_and_is_idempotent() {
    let home_dir = TempDir::new().unwrap();
    let skill_dir = skill_dir(home_dir.path());

    let missing = gitee_with_home(home_dir.path())
        .args(["skills", "uninstall"])
        .output()
        .unwrap();

    assert_eq!(missing.status.code(), Some(0));
    assert!(missing.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&missing.stdout).trim(),
        "using-gitee-cli is not installed"
    );

    let install = gitee_with_home(home_dir.path())
        .args(["skills", "install"])
        .output()
        .unwrap();
    assert_eq!(install.status.code(), Some(0));
    assert!(skill_dir.exists());

    let removed = gitee_with_home(home_dir.path())
        .args(["skills", "remove", "--json"])
        .output()
        .unwrap();

    assert_eq!(removed.status.code(), Some(0));
    assert!(removed.stderr.is_empty());
    assert!(!skill_dir.exists());

    let body: Value = serde_json::from_slice(&removed.stdout).unwrap();
    assert_eq!(body["name"], "using-gitee-cli");
    assert_eq!(body["agent"], "default");
    assert_eq!(body["installed"], false);
    assert_eq!(body["action"], "uninstalled");
    assert_eq!(body["path"], skill_dir.display().to_string());
}

#[test]
fn help_describes_skills_commands_and_json_metadata() {
    let text_output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "skills"])
        .output()
        .unwrap();

    assert_eq!(text_output.status.code(), Some(0));
    assert!(text_output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(stdout.contains("Manage the bundled using-gitee-cli skill"));
    assert!(stdout.contains("install"));
    assert!(stdout.contains("uninstall"));
    assert!(stdout.contains("list"));

    let direct_install_help = Command::cargo_bin("gitee")
        .unwrap()
        .args(["skills", "install", "--help"])
        .output()
        .unwrap();

    assert_eq!(direct_install_help.status.code(), Some(0));
    assert!(direct_install_help.stderr.is_empty());
    let direct_stdout = String::from_utf8_lossy(&direct_install_help.stdout);
    assert!(direct_stdout.contains("--json"));
    assert!(!direct_stdout.contains("--json [<FIELDS>]"));

    let install_json_output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "skills", "install", "--json"])
        .output()
        .unwrap();

    assert_eq!(install_json_output.status.code(), Some(0));
    assert!(install_json_output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&install_json_output.stdout).unwrap();
    assert_eq!(body["path"], "skills install");
    assert_eq!(body["gh_equivalent"], "not_applicable");
    assert_eq!(body["supports_json"], true);
    assert_eq!(body["auth"], "not_required");
    assert_eq!(
        body["notes"][0],
        "Omit --agent to install to ~/.agents/skills/using-gitee-cli."
    );

    for (topic, path) in [
        (
            ["help", "skills", "uninstall", "--json"],
            "skills uninstall",
        ),
        (["help", "skills", "remove", "--json"], "skills uninstall"),
        (["help", "skills", "list", "--json"], "skills list"),
        (["help", "skills", "ls", "--json"], "skills list"),
    ] {
        let output = Command::cargo_bin("gitee")
            .unwrap()
            .args(topic)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());

        let body: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(body["path"], path);
        assert_eq!(body["gh_equivalent"], "not_applicable");
        assert_eq!(body["supports_json"], true);
    }
}

#[test]
fn root_help_json_includes_the_skills_group() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .args(["help", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    let commands = body["commands"].as_array().unwrap();
    assert!(commands.iter().any(|command| command["name"] == "skills"));
}

#[test]
fn skills_commands_fail_with_config_error_when_home_cannot_be_resolved() {
    let output = Command::cargo_bin("gitee")
        .unwrap()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("HOMEDRIVE")
        .env_remove("HOMEPATH")
        .args(["skills", "list"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "config error: could not determine home directory for skills installation"
    );
}

#[test]
fn skills_install_with_agent_claude_code_writes_to_the_claude_code_skill_dir() {
    // Text output on a fresh home.
    let text_home = TempDir::new().unwrap();
    let text_dir = claude_code_skill_dir(text_home.path());
    let text = gitee_with_home(text_home.path())
        .args(["skills", "install", "--agent", "claude-code"])
        .output()
        .unwrap();

    assert_eq!(text.status.code(), Some(0));
    assert!(text.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&text.stdout).trim(),
        format!("installed using-gitee-cli to {}", text_dir.display())
    );
    assert!(text_dir.join("SKILL.md").is_file());
    assert!(text_dir.join("references/commands.md").is_file());
    assert!(!skill_dir(text_home.path()).exists());

    // JSON output and content parity on a separate home.
    let home_dir = TempDir::new().unwrap();
    let claude_dir = claude_code_skill_dir(home_dir.path());
    let output = gitee_with_home(home_dir.path())
        .args(["skills", "install", "--agent", "claude-code", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["name"], "using-gitee-cli");
    assert_eq!(body["agent"], "claude-code");
    assert_eq!(body["installed"], true);
    assert_eq!(body["action"], "installed");
    assert_eq!(body["path"], claude_dir.display().to_string());

    assert!(claude_dir.join("SKILL.md").is_file());
    assert!(claude_dir.join("references/commands.md").is_file());
    assert_eq!(read_tree(&source_skill_dir()), read_tree(&claude_dir));

    // The default target must remain untouched.
    assert!(!skill_dir(home_dir.path()).exists());
}

#[test]
fn skills_uninstall_with_agent_claude_code_removes_only_the_selected_target() {
    let home_dir = TempDir::new().unwrap();
    let default_dir = skill_dir(home_dir.path());
    let claude_dir = claude_code_skill_dir(home_dir.path());

    let install = gitee_with_home(home_dir.path())
        .args(["skills", "install", "--agent", "claude-code"])
        .output()
        .unwrap();
    assert_eq!(install.status.code(), Some(0));
    assert!(claude_dir.exists());

    // Uninstall with no flag removes only the default target (a no-op here),
    // leaving the Claude Code target in place.
    let removed = gitee_with_home(home_dir.path())
        .args(["skills", "uninstall"])
        .output()
        .unwrap();
    assert_eq!(removed.status.code(), Some(0));
    assert!(claude_dir.exists());

    // Uninstall with --agent claude-code removes only the Claude Code target.
    let removed_cc = gitee_with_home(home_dir.path())
        .args(["skills", "uninstall", "--agent", "claude-code", "--json"])
        .output()
        .unwrap();
    assert_eq!(removed_cc.status.code(), Some(0));
    assert!(!claude_dir.exists());

    let body: Value = serde_json::from_slice(&removed_cc.stdout).unwrap();
    assert_eq!(body["agent"], "claude-code");
    assert_eq!(body["action"], "uninstalled");

    // Reinstall and verify the text uninstall output for the claude-code target.
    let reinstall = gitee_with_home(home_dir.path())
        .args(["skills", "install", "--agent", "claude-code"])
        .output()
        .unwrap();
    assert_eq!(reinstall.status.code(), Some(0));

    let removed_cc_text = gitee_with_home(home_dir.path())
        .args(["skills", "uninstall", "--agent", "claude-code"])
        .output()
        .unwrap();
    assert_eq!(removed_cc_text.status.code(), Some(0));
    assert!(removed_cc_text.stderr.is_empty());
    assert!(!claude_dir.exists());
    assert_eq!(
        String::from_utf8_lossy(&removed_cc_text.stdout).trim(),
        format!("uninstalled using-gitee-cli from {}", claude_dir.display())
    );

    // The default target was never created by this sequence.
    assert!(!default_dir.exists());
}

#[test]
fn skills_rejects_an_invalid_or_ambiguous_agent_value() {
    let home_dir = TempDir::new().unwrap();

    for args in [
        vec!["skills", "install", "--agent", "claude"],
        vec!["skills", "list", "--agent", "nope"],
    ] {
        let output = gitee_with_home(home_dir.path())
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid value for --agent: expected claude-code"),
            "unexpected stderr: {stderr}"
        );
    }
}

fn gitee_with_home(home: &Path) -> Command {
    let mut command = Command::cargo_bin("gitee").unwrap();
    command
        .env("HOME", home)
        .env_remove("USERPROFILE")
        .env_remove("HOMEDRIVE")
        .env_remove("HOMEPATH");
    command
}

fn skill_dir(home: &Path) -> PathBuf {
    home.join(".agents/skills/using-gitee-cli")
}

fn claude_code_skill_dir(home: &Path) -> PathBuf {
    home.join(".claude/skills/using-gitee-cli")
}

fn source_skill_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/using-gitee-cli")
}

fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    read_tree_into(root, root, &mut files);
    files
}

fn read_tree_into(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            read_tree_into(root, &path, files);
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(relative_path, fs::read(path).unwrap());
    }
}
