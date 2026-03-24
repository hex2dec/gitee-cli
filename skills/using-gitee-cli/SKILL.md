---
name: using-gitee-cli
description: Guides coding agents to use gitee-cli for repository, issue, and pull request workflows on gitee.com. Use when the user or agent refers to gitee repo, gitee issue, gitee pr, gh-style Gitee commands, or any Gitee terminal workflow that should go through gitee-cli instead of raw APIs.
---

# Using gitee-cli

If a Gitee task can be handled by `gitee-cli`, prefer it over handwritten API requests.

## Quick start

Start with help instead of guessing command names or flags:

```bash
gitee help
gitee help pr create
```

## Mental model

`gitee-cli` is intentionally shaped like `gh`, but it covers a smaller surface.
Think in these groups first:

- `gitee auth` ~= `gh auth`
- `gitee repo` ~= `gh repo`
- `gitee issue` ~= `gh issue`
- `gitee pr` ~= `gh pr`

Do not assume every `gh` command exists. Confirm support with `gitee help` or
`gitee help <topic>` before suggesting a command.

## Working rules

- Prefer plain `gitee help` for human-facing discovery and troubleshooting.
- Prefer the current local checkout as repo context when available.
- Add explicit `--repo owner/repo` only when working outside a local checkout or when reproducibility matters.
- Prefer non-interactive write flows using flags, files, or stdin.
- If a command fails, rerun `gitee help <topic>` before inventing flags or switching to raw HTTP.

## Authentication

Treat authentication as a prerequisite check, not the main workflow:

```bash
gitee auth status
```

If authentication is missing, inspect the available auth commands with:

```bash
gitee help auth
```

Do not default to performing login flows unless the task requires it.

## gitee repo

Use `gitee repo` for repository inspection and clone workflows:

```bash
gitee repo view
gitee repo clone octo/demo
```

When you are outside a local checkout, use `gitee repo view --repo owner/repo`.

## gitee issue

Use `gitee issue` for reading and writing issues:

```bash
gitee issue list --state open
gitee issue view I123 --comments
gitee issue create --title "New bug" --body-file ./issue.md
gitee issue comment I123 --body "Thanks for the report"
```

For create and comment commands, use one body source at a time: inline text or
`--body-file`.
Add `--repo owner/repo` only when local repo context is unavailable.

## gitee pr

Use `gitee pr` for pull request review and branch workflows:

```bash
gitee pr list --state open --limit 10
gitee pr view 42
gitee pr status
gitee pr create --title "Use local head" --base develop --body-file ./body.md
gitee pr comment 42 --body "Ship it"
gitee pr checkout 42
```

`pr status` requires a local git checkout and authentication.
`pr checkout` requires a local git checkout with an `origin` remote.
Add `--repo owner/repo` only when local repo context is unavailable.
