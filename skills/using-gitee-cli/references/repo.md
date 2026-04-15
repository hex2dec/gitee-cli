# gitee-cli Repo Commands

Load this file for repository inspection and clone workflows.

## `gitee repo view`

- `gh` equivalent: `gh repo view`
- Summary: view repository metadata.
- Auth: `optional`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee repo view [--repo <OWNER/REPO>] [--json]
```

- Flags:
  - `--repo <OWNER/REPO>`: target repository; defaults to local git context when supported
  - `--json`: output machine-readable JSON
- Notes:
  - Use `--repo` when you are outside a local checkout.
- Examples:

```bash
gitee repo view --repo octo/demo --json
gitee repo view --json
```

## `gitee repo clone`

- `gh` equivalent: `gh repo clone`
- Summary: clone a repository by `OWNER/REPO` slug.
- Auth: `optional`
- Local git required: no
- Repo inference: no
- Syntax:

```bash
gitee repo clone <OWNER/REPO> [DESTINATION] [--https | --ssh] [--json]
```

- Arguments:
  - `<OWNER/REPO>`: repository slug to clone
  - `[DESTINATION]`: optional local destination directory
- Flags:
  - `--https`: clone over HTTPS
  - `--ssh`: clone over SSH
  - `--json`: output machine-readable JSON
- Notes:
  - Use at most one of `--https` or `--ssh`.
  - When neither is provided, the CLI uses a saved clone protocol preference or prompts on first use.
- Examples:

```bash
gitee repo clone octo/demo
gitee repo clone octo/demo demo-https --https --json
```
