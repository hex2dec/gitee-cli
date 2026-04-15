# gitee-cli Command Map

Use this file as the navigation entrypoint for `gitee-cli` command lookup.
After reading it, load only the smallest relevant group document instead of the
entire command set.

This reference is a Coding Agent-focused subset of the current CLI help surface.
It is verified against:

```bash
gitee help --json
gitee help <topic>
gitee help <topic> --json
```

It intentionally omits `gitee auth login` and `gitee auth logout`, because
Coding Agents should treat credential management as an external prerequisite
instead of performing token save/remove flows themselves.

If the installed binary disagrees with this file, trust the live help output.

## Discovery first

Start with help instead of guessing:

```bash
gitee help
gitee help --json
gitee help pr create
gitee help pr create --json
```

Use `gitee help --json` when an agent needs a machine-readable manifest of:

- supported command groups
- subcommands
- flags
- examples
- auth requirements
- local git requirements
- `gh`-style equivalents

## Core rules

- Prefer the current local checkout as repository context when available.
- Add `--repo owner/repo` when you are outside a checkout or reproducibility matters.
- Prefer bare `--json` for machine-readable output.
- For write commands, do not invent prompts or interactive flows when a flag, file, or stdin path exists.
- For `--body` and `--body-file`, use at most one body source unless the command explicitly says otherwise.
- Treat authentication setup as user-owned. Check auth state, but do not suggest saving or clearing tokens unless the user explicitly asks about credential management.
- If a command fails because of missing flags or unsupported behavior, rerun `gitee help <topic>` before suggesting alternatives.

## Top-level groups

| Group | `gh` equivalent | Purpose |
| --- | --- | --- |
| `gitee auth` | `gh auth` | Authentication prerequisite checks |
| `gitee repo` | `gh repo` | Repository inspection and clone |
| `gitee issue` | `gh issue` | Issue list, view, create, comment |
| `gitee pr` | `gh pr` | Pull request list, view, create, edit, comment, status, checkout |

## Reference map

Load only one of these unless the task spans multiple areas:

- [`auth.md`](auth.md): `gitee auth status` only. Use for auth prerequisite checks.
- [`repo.md`](repo.md): `gitee repo view` and `gitee repo clone`.
- [`issue.md`](issue.md): `gitee issue list`, `view`, `create`, `comment`.
- [`pr.md`](pr.md): `gitee pr list`, `view`, `create`, `edit`, `comment`, `status`, `checkout`.

Recommended lookup path:

1. Read this file.
2. Pick the command group that matches the user task.
3. Load only that group file.
4. Fall back to `gitee help <topic>` or `gitee help <topic> --json` if exact flags are still unclear.

Current unsupported `gh`-style areas exposed by `gitee help --json`:

- `api`
- `release`
- `label`
- `workflow`
- `notification`

## Context and requirement model

Use these terms consistently when suggesting commands:

- `auth: not_required`: command can run without authentication.
- `auth: optional`: command can often run without authentication, but auth may improve or unlock access.
- `auth: required`: command needs usable authentication.
- `local git required`: command needs a local git checkout.
- `repo inference`: command can infer `owner/repo` from the current checkout when `--repo` is omitted.

Common rules:

- Most read and write repo/issue/pr commands support repo inference.
- `repo clone` never uses repo inference; you must provide `OWNER/REPO`.
- `pr status` requires a local git checkout and authentication.
- `pr checkout` requires a local git checkout with an `origin` remote.
- This skill intentionally exposes only `gitee auth status` from the `auth` group.
