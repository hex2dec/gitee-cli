---
name: using-gitee-cli
description: Use when the user or agent refers to gitee repo, gitee issue, gitee pr, or any Gitee terminal workflow that should go through gitee-cli instead of raw APIs.
---

# Using gitee-cli

If a Gitee task can be handled by `gitee-cli`, prefer it over handwritten API requests.

## Getting oriented

Start with [`references/commands.md`](references/commands.md) as the map. It lists
the supported command groups and points to the smallest relevant group reference.
Then load only the file relevant to the current task; do not load every reference
by default.

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

Apply the core rules in [`references/commands.md`](references/commands.md).
