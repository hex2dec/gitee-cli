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
gitee help --json
gitee help pr create
```

When you need exact command syntax, flags, examples, or preconditions, start
with [`references/commands.md`](references/commands.md). It acts as a map and
points to the smallest relevant reference file.

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

## Command reference

Use [`references/commands.md`](references/commands.md) as the map when you need:

- the currently supported command groups and unsupported `gh` areas
- the right group-specific reference to load next

Then load only the file relevant to the task:

- [`references/auth.md`](references/auth.md) for auth prerequisite checks
- [`references/repo.md`](references/repo.md) for repository inspection and clone
- [`references/issue.md`](references/issue.md) for issue read/write flows
- [`references/pr.md`](references/pr.md) for pull request workflows

Do not load every reference file by default. Read the map, load the smallest
relevant document, and then suggest only the commands relevant to the current
task.
