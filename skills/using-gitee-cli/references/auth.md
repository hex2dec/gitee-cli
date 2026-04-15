# gitee-cli Auth Commands

Load this file only when the task is about checking whether authentication is
already usable. This skill intentionally does not document token save/remove
flows for Coding Agents.

## `gitee auth status`

- `gh` equivalent: `gh auth status`
- Summary: check whether authentication is currently usable.
- Auth: `not_required`
- Local git required: no
- Repo inference: no
- Syntax:

```bash
gitee auth status [--json]
```

- Flags:
  - `--json`: output machine-readable JSON
- Notes:
  - Reads the token from `GITEE_TOKEN` first, then the saved config file.
- Examples:

```bash
gitee auth status
gitee auth status --json
```
