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
- Use explicit `--repo owner/repo` in scripts or when reproducibility matters.
- Omit `--repo` only when the current local checkout should supply repo context.
- Prefer non-interactive write flows using flags, files, or stdin.
- If a command fails, rerun `gitee help <topic>` before inventing flags or switching to raw HTTP.

## Common workflows

Check auth:

```bash
gitee auth status
gitee auth login --token "$GITEE_TOKEN"
printf '%s\n' "$TOKEN" | gitee auth login --with-token
```

Inspect repositories:

```bash
gitee repo view --repo octo/demo
gitee repo view
gitee repo clone octo/demo
```

Read and write issues:

```bash
gitee issue list --repo octo/demo --state open
gitee issue view I123 --repo octo/demo --comments
gitee issue create --repo octo/demo --title "New bug" --body-file ./issue.md
gitee issue comment I123 --repo octo/demo --body "Thanks for the report"
```

Work with pull requests:

```bash
gitee pr list --repo octo/demo --state open --limit 10
gitee pr view 42 --repo octo/demo
gitee pr status
gitee pr create --title "Use local head" --base develop --body-file ./body.md
gitee pr comment 42 --repo octo/demo --body "Ship it"
gitee pr checkout 42 --repo octo/demo
```

## Guardrails

- `pr status` requires a local git checkout and authentication.
- `pr checkout` requires a local git checkout with an `origin` remote.
- Repo inference is convenient, but explicit `--repo` is safer for automation.
- `auth login` accepts exactly one of `--token` or `--with-token`.
- For create/comment commands, use one body source at a time: inline text or `--body-file`.
