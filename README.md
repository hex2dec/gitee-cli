# gitee-cli

`gitee-cli` is a small Rust command-line tool for Gitee authentication, repository inspection and cloning, issue workflows, and pull request workflows.

The installed executable is named `gitee`.

## Quick Start

Build the binary locally:

```bash
cargo build
```

Run commands from the repository during development:

```bash
cargo run -- auth status --json
```

### Authentication

Check whether a usable token is available:

```bash
gitee auth status --json
```

Validate and save a token from a flag:

```bash
gitee auth login --token "$GITEE_TOKEN" --json
```

Validate and save a token from stdin:

```bash
printf '%s\n' "$TOKEN" | gitee auth login --with-token --json
```

Clear the saved token:

```bash
gitee auth logout --json
```

### Repository Commands

View repository metadata by explicit slug:

```bash
gitee repo view --repo octo/demo --json
```

Inside a local Gitee checkout, infer the repository from `origin`:

```bash
gitee repo view --json
```

Clone over HTTPS by default:

```bash
gitee repo clone octo/demo
```

Clone over SSH to an explicit destination:

```bash
gitee repo clone octo/demo demo-ssh --ssh --json
```

### Issue Commands

List issues for the current local repository:

```bash
gitee issue list --state open --page 1 --per-page 20 --json
```

List issues for an explicit repository with filters:

```bash
gitee issue list --repo octo/demo --state closed --search panic --page 2 --per-page 5 --json
```

View a single issue:

```bash
gitee issue view I123 --repo octo/demo --json
```

Include paginated comment history when viewing an issue:

```bash
gitee issue view I123 --repo octo/demo --comments --page 2 --per-page 10 --json
```

Post an issue comment from a flag, file, or stdin:

```bash
gitee issue comment I123 --repo octo/demo --body "Thanks for the report" --json
gitee issue comment I123 --repo octo/demo --body-file ./comment.txt
printf '%s' "Posted from stdin" | gitee issue comment I123 --repo octo/demo --body-stdin --json
```

### Pull Request Commands

View a pull request:

```bash
gitee pr view 42 --repo octo/demo --json
```

List pull requests with filters:

```bash
gitee pr list --repo octo/demo --state open --author octocat --assignee reviewer --base main --head feature/pr-list --limit 10 --json
```

Show the current branch PRs plus PRs authored by or assigned to the current user:

```bash
gitee pr status --state open --limit 10 --json
```

Create a pull request with an explicit head:

```bash
gitee pr create --repo octo/demo --head feature/pr-create --base main --title "Add PR create" --body "Creates the pull request" --json
```

Create a pull request from the current local branch, letting `gitee-cli` infer `--head` and the repository:

```bash
gitee pr create --title "Use local head" --base develop --body "Built from the local branch"
```

Read a PR body from a file or from stdin:

```bash
gitee pr create --repo octo/demo --head feature/body-file --title "Read body file" --body-file ./body.md --json
printf '%s\n' "Generated from stdin" | gitee pr create --repo octo/demo --head feature/stdin --base main --title "Read stdin" --body-file - --json
```

Comment on a pull request:

```bash
gitee pr comment 42 --repo octo/demo --body "Ship it" --json
gitee pr comment 42 --repo octo/demo --body-file ./review.md
```

Check out the pull request head branch into the current local repository:

```bash
gitee pr checkout 42 --repo octo/demo --json
```

## Repository Context Inference

When `--repo` is omitted, `gitee-cli` tries to infer the repository from the local git checkout by reading:

- the current branch
- the `origin` remote URL

Supported `origin` URL forms are:

- `git@gitee.com:owner/repo.git`
- `ssh://git@gitee.com/owner/repo.git`
- `https://gitee.com/owner/repo.git`
- `http://gitee.com/owner/repo.git`

Commands that can use local repository context include:

- `repo view`
- `issue list`
- `issue view`
- `issue comment`
- `pr view`
- `pr list`
- `pr comment`
- `pr create`
- `pr checkout`
- `pr status`

`pr status` always requires a local git checkout. `pr checkout` also requires a local git checkout with an `origin` remote.

## Authentication and Configuration

Token resolution order:

1. `GITEE_TOKEN`
2. saved config file token

Config directory resolution order:

1. `GITEE_CONFIG_DIR`
2. `XDG_CONFIG_HOME/gitee`
3. `HOME/.config/gitee`
4. current directory `./.gitee`

Relevant environment variables:

- `GITEE_TOKEN`: overrides the saved token at runtime
- `GITEE_CONFIG_DIR`: points directly to the config directory
- `XDG_CONFIG_HOME`: used when `GITEE_CONFIG_DIR` is not set
- `HOME`: used for the default config path
- `GITEE_BASE_URL`: overrides the API base URL, which defaults to `https://gitee.com/api`; mainly useful for tests

By default the saved token lives at `~/.config/gitee/config.toml`.

Commands that require authentication:

- `auth login`
- `issue comment`
- `pr comment`
- `pr create`
- `pr status`

Other commands can run without a token for public repositories, but private repositories and human-name fallback lookups may still require authentication.

## Exit Codes

- `0`: success
- `2`: usage error
- `3`: authentication error or authentication required
- `4`: config error
- `5`: remote request error
- `6`: resource not found
- `7`: local git error

## Development

The project uses Rust `1.94.0` from [`.tool-versions`](./.tool-versions).

```bash
cargo build
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Useful smoke tests:

```bash
cargo run -- auth status --json
cargo run -- repo view --repo octo/demo --json
cargo run -- issue list --repo octo/demo --json
cargo run -- pr list --repo octo/demo --json
```

## License

MIT. See [LICENSE](./LICENSE).
