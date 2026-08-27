use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{Dir, DirEntry, include_dir};

use crate::command::{CommandError, CommandOutcome, EXIT_OK, OutputFormat};
use crate::config::home_dir;

const SKILL_NAME: &str = "using-gitee-cli";
static BUNDLED_SKILL_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills/using-gitee-cli");

/// The install target for the bundled skill, selected via `--agent`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    /// The cross-client Agent Skills standard directory (`~/.agents/skills`).
    Default,
    /// Claude Code's personal skill directory (`~/.claude/skills`).
    ClaudeCode,
}

impl AgentKind {
    /// All known targets in deterministic (output) order: default first.
    pub const ALL: [AgentKind; 2] = [AgentKind::Default, AgentKind::ClaudeCode];

    /// Parse a `--agent` value. Only `claude-code` is supported; any other value is a usage error.
    /// The `claude` alias is intentionally rejected because it reads as the Agent SDK/`.agents`
    /// convention in this ecosystem.
    pub fn parse(value: &str) -> Result<Self, CommandError> {
        match value {
            "claude-code" => Ok(Self::ClaudeCode),
            _ => Err(CommandError::usage(
                "invalid value for --agent: expected claude-code (omit the flag for the default cross-client target)",
            )),
        }
    }

    /// The value reported in JSON `agent` fields.
    pub fn json_label(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ClaudeCode => "claude-code",
        }
    }

    /// The text label used in `list` output rows.
    fn list_label(&self) -> &'static str {
        match self {
            Self::Default => "(default)",
            Self::ClaudeCode => "claude-code",
        }
    }

    /// The home-relative directory this target installs into.
    fn rel_dir(&self) -> &'static str {
        match self {
            Self::Default => ".agents/skills/using-gitee-cli",
            Self::ClaudeCode => ".claude/skills/using-gitee-cli",
        }
    }
}

enum SkillAction {
    Installed,
    Updated,
    Uninstalled,
    Noop,
}

pub struct SkillsService {
    home_dir: PathBuf,
}

impl SkillsService {
    pub fn from_env() -> Result<Self, CommandError> {
        let home_dir = home_dir().ok_or_else(|| {
            CommandError::config("could not determine home directory for skills installation")
        })?;

        Ok(Self { home_dir })
    }

    pub fn install(
        &self,
        agent: AgentKind,
        output: OutputFormat,
    ) -> Result<CommandOutcome, CommandError> {
        let target_dir = self.target_dir(agent);
        let existed = target_exists(&target_dir);
        if existed {
            remove_existing_target(&target_dir)?;
        }

        fs::create_dir_all(&target_dir).map_err(CommandError::config)?;
        write_bundled_skill_dir(&BUNDLED_SKILL_DIR, &target_dir)?;

        let action = if existed {
            SkillAction::Updated
        } else {
            SkillAction::Installed
        };
        Ok(self.render_mutation(agent, output, action, true))
    }

    pub fn uninstall(
        &self,
        agent: AgentKind,
        output: OutputFormat,
    ) -> Result<CommandOutcome, CommandError> {
        let target_dir = self.target_dir(agent);
        if !target_exists(&target_dir) {
            return Ok(self.render_mutation(agent, output, SkillAction::Noop, false));
        }

        remove_existing_target(&target_dir)?;
        Ok(self.render_mutation(agent, output, SkillAction::Uninstalled, false))
    }

    pub fn list(&self, agent: Option<AgentKind>, output: OutputFormat) -> CommandOutcome {
        let targets: Vec<AgentKind> = match agent {
            Some(agent) => vec![agent],
            None => AgentKind::ALL.to_vec(),
        };

        match output {
            OutputFormat::Text => {
                let rows = targets
                    .into_iter()
                    .map(|target| {
                        let installed = target_exists(&self.target_dir(target));
                        let status = if installed {
                            "installed"
                        } else {
                            "not installed"
                        };
                        format!(
                            "{SKILL_NAME} {}\t{status}\t{}",
                            target.list_label(),
                            self.target_dir(target).display()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                CommandOutcome::text(EXIT_OK, rows)
            }
            OutputFormat::Json { .. } => {
                let entries = targets
                    .into_iter()
                    .map(|target| {
                        serde_json::json!({
                            "name": SKILL_NAME,
                            "agent": target.json_label(),
                            "installed": target_exists(&self.target_dir(target)),
                            "path": self.target_dir(target).display().to_string(),
                        })
                    })
                    .collect::<Vec<_>>();
                CommandOutcome::json(EXIT_OK, serde_json::Value::Array(entries))
            }
        }
    }

    fn render_mutation(
        &self,
        agent: AgentKind,
        output: OutputFormat,
        action: SkillAction,
        installed: bool,
    ) -> CommandOutcome {
        let path = self.target_dir(agent).display().to_string();
        match output {
            OutputFormat::Text => CommandOutcome::text(EXIT_OK, action.text(&path)),
            OutputFormat::Json { .. } => CommandOutcome::json(
                EXIT_OK,
                serde_json::json!({
                    "name": SKILL_NAME,
                    "agent": agent.json_label(),
                    "installed": installed,
                    "action": action.as_str(),
                    "path": path,
                }),
            ),
        }
    }

    fn target_dir(&self, agent: AgentKind) -> PathBuf {
        self.home_dir.join(agent.rel_dir())
    }
}

impl SkillAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Updated => "updated",
            Self::Uninstalled => "uninstalled",
            Self::Noop => "noop",
        }
    }

    fn text(&self, path: &str) -> String {
        match self {
            Self::Installed => format!("installed {SKILL_NAME} to {path}"),
            Self::Updated => format!("updated {SKILL_NAME} at {path}"),
            Self::Uninstalled => format!("uninstalled {SKILL_NAME} from {path}"),
            Self::Noop => format!("{SKILL_NAME} is not installed"),
        }
    }
}

fn target_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn remove_existing_target(path: &Path) -> Result<(), CommandError> {
    let metadata = fs::symlink_metadata(path).map_err(CommandError::config)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(CommandError::config)
    } else {
        fs::remove_file(path).map_err(CommandError::config)
    }
}

fn write_bundled_skill_dir(dir: &Dir<'_>, target_dir: &Path) -> Result<(), CommandError> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(dir) => write_bundled_skill_dir(dir, target_dir)?,
            DirEntry::File(file) => {
                let path = target_dir.join(file.path());
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(CommandError::config)?;
                }
                fs::write(path, file.contents()).map_err(CommandError::config)?;
            }
        }
    }

    Ok(())
}
