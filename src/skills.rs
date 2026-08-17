use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{Dir, DirEntry, include_dir};

use crate::command::{CommandError, CommandOutcome, EXIT_OK, OutputFormat};
use crate::config::home_dir;

const SKILL_NAME: &str = "using-gitee-cli";
const SKILL_DIR_RELATIVE: &str = ".agents/skills/using-gitee-cli";
static BUNDLED_SKILL_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills/using-gitee-cli");

enum SkillAction {
    Installed,
    Updated,
    Uninstalled,
    Noop,
}

pub struct SkillsService {
    target_dir: PathBuf,
}

impl SkillsService {
    pub fn from_env() -> Result<Self, CommandError> {
        let home_dir = home_dir().ok_or_else(|| {
            CommandError::config("could not determine home directory for skills installation")
        })?;

        Ok(Self {
            target_dir: home_dir.join(SKILL_DIR_RELATIVE),
        })
    }

    pub fn install(&self, output: OutputFormat) -> Result<CommandOutcome, CommandError> {
        let existed = self.target_exists();
        if existed {
            remove_existing_target(&self.target_dir)?;
        }

        fs::create_dir_all(&self.target_dir).map_err(CommandError::config)?;
        write_bundled_skill_dir(&BUNDLED_SKILL_DIR, &self.target_dir)?;

        let action = if existed {
            SkillAction::Updated
        } else {
            SkillAction::Installed
        };
        Ok(self.render_mutation(output, action, true))
    }

    pub fn uninstall(&self, output: OutputFormat) -> Result<CommandOutcome, CommandError> {
        if !self.target_exists() {
            return Ok(self.render_mutation(output, SkillAction::Noop, false));
        }

        remove_existing_target(&self.target_dir)?;
        Ok(self.render_mutation(output, SkillAction::Uninstalled, false))
    }

    pub fn list(&self, output: OutputFormat) -> CommandOutcome {
        let installed = self.target_exists();
        let path = self.target_path();

        match output {
            OutputFormat::Text => {
                let status = if installed {
                    "installed"
                } else {
                    "not installed"
                };
                CommandOutcome::text(EXIT_OK, format!("{SKILL_NAME}\t{status}\t{path}"))
            }
            OutputFormat::Json { .. } => CommandOutcome::json(
                EXIT_OK,
                serde_json::json!([
                    {
                        "name": SKILL_NAME,
                        "installed": installed,
                        "path": path,
                    }
                ]),
            ),
        }
    }

    fn render_mutation(
        &self,
        output: OutputFormat,
        action: SkillAction,
        installed: bool,
    ) -> CommandOutcome {
        let path = self.target_path();
        match output {
            OutputFormat::Text => CommandOutcome::text(EXIT_OK, action.text(&path)),
            OutputFormat::Json { .. } => CommandOutcome::json(
                EXIT_OK,
                serde_json::json!({
                    "name": SKILL_NAME,
                    "installed": installed,
                    "action": action.as_str(),
                    "path": path,
                }),
            ),
        }
    }

    fn target_exists(&self) -> bool {
        fs::symlink_metadata(&self.target_dir).is_ok()
    }

    fn target_path(&self) -> String {
        self.target_dir.display().to_string()
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
