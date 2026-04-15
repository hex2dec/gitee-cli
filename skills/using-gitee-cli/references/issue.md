# gitee-cli Issue Commands

Load this file for issue read/write flows.

## `gitee issue list`

- `gh` equivalent: `gh issue list`
- Summary: list issues for a repository.
- Auth: `optional`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee issue list [--repo <OWNER/REPO>] [--state <STATE>] [--search <TEXT>] [--page <N>] [--per-page <N>] [--json]
```

- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--state <STATE>`: `open`, `closed`, or `all`
  - `--search <TEXT>`: filter by keyword text
  - `--page <N>`: 1-based page number
  - `--per-page <N>`: results per page
  - `--json`: output machine-readable JSON
- Notes:
  - When `--repo` is omitted, the command can infer the repository from local git context.
- Examples:

```bash
gitee issue list --repo octo/demo --state open --json
gitee issue list --state open --page 1 --per-page 20 --json
```

## `gitee issue view`

- `gh` equivalent: `gh issue view`
- Summary: view a single issue and optionally include comments.
- Auth: `optional`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee issue view <ISSUE> [--repo <OWNER/REPO>] [--comments] [--page <N>] [--per-page <N>] [--json]
```

- Arguments:
  - `<ISSUE>`: issue number or identifier such as `I123`
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--comments`: include issue comments
  - `--page <N>`: comment page number
  - `--per-page <N>`: comment page size
  - `--json`: output machine-readable JSON
- Notes:
  - Comments are fetched only when `--comments` is provided.
- Examples:

```bash
gitee issue view I123 --repo octo/demo --json
gitee issue view I123 --comments --page 1 --per-page 20 --json
```

## `gitee issue create`

- `gh` equivalent: `gh issue create`
- Summary: create a new issue.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee issue create [--repo <OWNER/REPO>] --title <TITLE> [--body <TEXT> | --body-file <PATH>] [--json]
```

- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--title <TITLE>`: required issue title
  - `--body <TEXT>`: inline issue body
  - `--body-file <PATH>`: read issue body from a file
  - `--json`: output machine-readable JSON
- Notes:
  - `--title` is required.
  - Provide at most one of `--body` or `--body-file`.
  - When `--repo` is omitted, the command can infer the repository from local git context.
- Examples:

```bash
gitee issue create --repo octo/demo --title "New bug" --body "Steps to reproduce" --json
gitee issue create --title "New bug" --body-file ./issue.md --json
```

## `gitee issue comment`

- `gh` equivalent: `gh issue comment`
- Summary: post a comment to an issue.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee issue comment <ISSUE> [--repo <OWNER/REPO>] [--body <TEXT> | --body-file <PATH>] [--json]
```

- Arguments:
  - `<ISSUE>`: issue number or identifier such as `I123`
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--body <TEXT>`: inline comment body
  - `--body-file <PATH>`: read comment body from a file
  - `--json`: output machine-readable JSON
- Notes:
  - Provide exactly one of `--body` or `--body-file`.
  - When `--repo` is omitted, the command can infer the repository from local git context.
- Examples:

```bash
gitee issue comment I123 --repo octo/demo --body "Thanks for the report" --json
gitee issue comment I123 --body-file ./comment.md --json
```
