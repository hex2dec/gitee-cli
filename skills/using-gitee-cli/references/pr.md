# gitee-cli Pull Request Commands

Load this file for pull request workflows.

Unless noted, every command here supports `--json` and infers `--repo` from the
current local checkout. The exception is `gitee pr status`, which never infers
and requires a local checkout plus authentication.

## `gitee pr list`

- `gh` equivalent: `gh pr list`
- Summary: list pull requests with filters.
- Auth: `optional`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr list [--repo <OWNER/REPO>] [--state <STATE>] [--author <LOGIN>] [--assignee <LOGIN>] [--base <BRANCH>] [--head <BRANCH>] [--limit <N>] [--json]
```

- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--state <STATE>`: `open`, `closed`, `merged`, or `all`
  - `--author <LOGIN>`: filter by author login
  - `--assignee <LOGIN>`: filter by assignee login
  - `--base <BRANCH>`: filter by base branch
  - `--head <BRANCH>`: filter by head branch
  - `--limit <N>`: maximum number of PRs to return
- Examples:

```bash
gitee pr list --repo octo/demo --state open --author octocat --limit 10 --json
gitee pr list --state open --limit 10 --json
```

## `gitee pr view`

- `gh` equivalent: `gh pr view`
- Summary: view a single pull request.
- Auth: `optional`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr view <PR> [--repo <OWNER/REPO>] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
- Examples:

```bash
gitee pr view 42 --repo octo/demo --json
gitee pr view 42 --json
```

## `gitee pr create`

- `gh` equivalent: `gh pr create`
- Summary: create a pull request from the current branch or an explicit head.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr create [--repo <OWNER/REPO>] [--head <BRANCH>] [--base <BRANCH>] --title <TITLE> [--body <TEXT> | --body-file <PATH>] [--json]
```

- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--head <BRANCH>`: head branch instead of the current branch
  - `--base <BRANCH>`: base branch to target
  - `--title <TITLE>`: required pull request title
  - `--body <TEXT>`: inline body
  - `--body-file <PATH>`: read body from a file
- Notes:
  - `--title` is required.
  - Provide at most one of `--body` or `--body-file`.
- Examples:

```bash
gitee pr create --title "Use local head" --base develop --body "Built from the local branch"
gitee pr create --repo octo/demo --head feature/body-file --title "Read body file" --body-file ./body.md --json
```

## `gitee pr edit`

- `gh` equivalent: `gh pr edit`
- Summary: edit an existing pull request.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr edit <PR> [--repo <OWNER/REPO>] [--title <TITLE>] [--body <TEXT> | --body-file <PATH>] [--state <STATE>] [--draft | --ready] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--title <TITLE>`: replace the title
  - `--body <TEXT>`: replace the body
  - `--body-file <PATH>`: replace the body from a file
  - `--state <STATE>`: `open` or `closed`
  - `--draft`: mark as draft
  - `--ready`: mark as ready
- Notes:
  - Provide at least one of `--title`, `--body`, `--body-file`, `--state`, `--draft`, or `--ready`.
  - Provide at most one of `--body` or `--body-file`.
  - Provide at most one of `--draft` or `--ready`.
- Examples:

```bash
gitee pr edit 42 --repo octo/demo --title "Updated title" --json
gitee pr edit 42 --body-file ./body.md --state open --ready --json
```

## `gitee pr merge`

- `gh` equivalent: `gh pr merge`
- Summary: merge a pull request.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr merge <PR> [--repo <OWNER/REPO>] [--merge | --squash | --rebase] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--merge`: merge the pull request
  - `--squash`: squash and merge the pull request
  - `--rebase`: rebase and merge the pull request
- Notes:
  - Provide at most one of `--merge`, `--squash`, or `--rebase`.
  - When no merge strategy flag is provided, the command uses the default merge strategy.
- Examples:

```bash
gitee pr merge 42 --repo octo/demo --json
gitee pr merge 42 --squash --json
```

## `gitee pr comment`

- `gh` equivalent: `gh pr comment`
- Summary: post a comment to a pull request.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr comment <PR> [--repo <OWNER/REPO>] [--body <TEXT> | --body-file <PATH>] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `--body <TEXT>`: inline comment body
  - `--body-file <PATH>`: read comment body from a file
- Notes:
  - Provide exactly one of `--body` or `--body-file`.
- Examples:

```bash
gitee pr comment 42 --repo octo/demo --body "Ship it" --json
gitee pr comment 42 --body-file ./comment.md --json
```

## `gitee pr review`

- `gh` equivalent: `gh pr review`
- Summary: approve a pull request or post review feedback.
- Auth: `required`
- Local git required: no
- Repo inference: yes
- Syntax:

```bash
gitee pr review <PR> [--repo <OWNER/REPO>] (--approve | --comment) [--body <TEXT> | --body-file <PATH>] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
  - `-a, --approve`: approve the pull request
  - `-c, --comment`: post review feedback
  - `-b, --body <TEXT>`: inline review body
  - `-F, --body-file <PATH>`: read review body from a file; use `-` for stdin
- Notes:
  - Provide exactly one of `--approve` or `--comment`.
  - Comment reviews require exactly one body source.
  - Approval reviews do not accept `--body` or `--body-file`.
  - Gitee does not expose a GitHub-style request-changes review state, so this command does not provide `--request-changes`.
- Examples:

```bash
gitee pr review 42 --repo octo/demo --approve --json
gitee pr review 42 --comment --body "Looks good" --json
printf 'Generated review\n' | gitee pr review 42 -c -F - --json
```

## `gitee pr status`

- `gh` equivalent: `gh pr status`
- Summary: show pull requests related to the current local checkout.
- Auth: `required`
- Local git required: yes
- Repo inference: no
- Syntax:

```bash
gitee pr status [--state <STATE>] [--limit <N>] [--json]
```

- Flags:
  - `--state <STATE>`: `open`, `closed`, `merged`, or `all`
  - `--limit <N>`: maximum number of PRs to return
- Notes:
  - Requires a local git checkout and authentication.
- Examples:

```bash
gitee pr status --state open --limit 10 --json
gitee pr status --json
```

## `gitee pr checkout`

- `gh` equivalent: `gh pr checkout`
- Summary: fetch and check out a pull request head branch.
- Auth: `optional`
- Local git required: yes
- Repo inference: yes
- Syntax:

```bash
gitee pr checkout <PR> [--repo <OWNER/REPO>] [--json]
```

- Arguments:
  - `<PR>`: pull request number
- Flags:
  - `--repo <OWNER/REPO>`: target repository
- Notes:
  - Requires a local git checkout with an `origin` remote.
- Examples:

```bash
gitee pr checkout 42 --repo octo/demo --json
gitee pr checkout 42 --json
```
