# Agent-First Workflows for gitee.com

`gitee-cli` is an agent-first command-line tool for working with `gitee.com`
from scripts, local terminals, and AI-driven workflows.

It gives you a small, stable command surface for authentication, repository
inspection, issue triage, and pull request workflows without dropping down to
raw Gitee API calls.

The installed executable is named `gitee`.

For agent and LLM discovery, start with:

```bash
gitee help --json
```

This returns a machine-readable manifest of supported command groups,
subcommands, flags, examples, and `gh`-style equivalents. To inspect one
command only, use a topic path such as `gitee help pr create --json`.

> `gitee-cli` is an unofficial community project. It is not affiliated with,
> endorsed by, or sponsored by Gitee or `gitee.com`.
>
> Gitee and `gitee.com` are trademarks or registered trademarks of their
> respective owner. They are referenced here only to identify platform
> compatibility.

## Why This Exists

`gitee-cli` is built for the high-frequency Gitee tasks that show up in
automation and day-to-day development:

- checking whether auth is usable before starting work
- inspecting repository metadata from a slug or a local checkout
- reading issue history before making a change
- viewing, listing, creating, commenting on, and checking out pull requests
- producing stable `--json` output and meaningful exit codes for scripts

The project is intentionally opinionated:

- it targets `gitee.com`
- it prefers explicit, non-interactive workflows
- it supports both human-readable output and stable `--json`
- it uses local git context when that makes common workflows faster

## Who It Is For

Use `gitee-cli` if you want:

- a Gitee workflow tool that fits AI agents and automation
- a terminal-friendly way to inspect repos, issues, and pull requests
- write operations that accept flags, files, or stdin instead of prompts
- predictable behavior that can be scripted safely

## Install

Tagged GitHub releases publish prebuilt binaries for:

- Apple Silicon macOS: `aarch64-apple-darwin`
- Linux x86_64: `x86_64-unknown-linux-musl`

Download the matching archive from the GitHub Releases page, extract it, and
place `gitee` somewhere on your `PATH`.

Each release also includes `gitee-<version>-checksums.txt`.

## Build From Source

If you need a development build or a platform outside the published release
assets, build from source:

Build the binary locally:

```bash
cargo build
```

Run commands from the repository during development:

```bash
cargo run -- auth status --json
```

Authenticate with a token:

```bash
gitee auth login --token "$GITEE_TOKEN" --json
```

Check that auth is usable:

```bash
gitee auth status --json
```

Inspect a repository directly:

```bash
gitee repo view --repo octo/demo --json
```

Or, inside a local Gitee checkout, let `gitee-cli` infer the repository from
`origin`:

```bash
gitee repo view --json
```

If you only try one end-to-end workflow, start here:

1. Build the CLI.
2. Log in with a personal access token.
3. Run `gitee repo view --repo octo/demo --json`.
4. Move into a local checkout and run `gitee pr status --state open --limit 10 --json`.

## Common Workflows

### Check Authentication Before Doing Work

Use `auth status` when a script or agent needs to fail fast before it touches a
repository or API:

```bash
gitee auth status --json
```

To save a token from stdin instead of a flag:

```bash
printf '%s\n' "$TOKEN" | gitee auth login --with-token --json
```

### Inspect a Repository Quickly

When you know the repository slug:

```bash
gitee repo view --repo octo/demo --json
```

When you are already inside a local checkout:

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

### Read Issue Context Before Making a Change

List open issues for the current repository:

```bash
gitee issue list --state open --page 1 --per-page 20 --json
```

View one issue in an explicit repository:

```bash
gitee issue view I123 --repo octo/demo --json
```

Include comment history when you need prior discussion:

```bash
gitee issue view I123 --repo octo/demo --comments --page 1 --per-page 20 --json
```

Post a follow-up comment non-interactively:

```bash
gitee issue comment I123 --repo octo/demo --body "Thanks for the report" --json
```

### Work with Pull Requests Without Leaving the Terminal

View a pull request:

```bash
gitee pr view 42 --repo octo/demo --json
```

List pull requests with filters:

```bash
gitee pr list --repo octo/demo --state open --author octocat --limit 10 --json
```

Show the pull requests related to the current branch or current user:

```bash
gitee pr status --state open --limit 10 --json
```

Create a pull request from the current branch:

```bash
gitee pr create --title "Use local head" --base develop --body "Built from the local branch"
```

Read a PR body from a file:

```bash
gitee pr create --repo octo/demo --head feature/body-file --title "Read body file" --body-file ./body.md --json
```

Comment on a pull request:

```bash
gitee pr comment 42 --repo octo/demo --body "Ship it" --json
```

Check out a pull request head branch into the current local repository:

```bash
gitee pr checkout 42 --repo octo/demo --json
```

## Local Repository Context

When `--repo` is omitted, `gitee-cli` tries to infer the repository from the
local git checkout. That keeps common commands short when you are already in the
right repository.

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

`pr status` always requires a local git checkout. `pr checkout` also requires a
local git checkout with an `origin` remote.

<details>
<summary>Supported <code>origin</code> URL forms</summary>

- `git@gitee.com:owner/repo.git`
- `ssh://git@gitee.com/owner/repo.git`
- `https://gitee.com/owner/repo.git`
- `http://gitee.com/owner/repo.git`

</details>

## Authentication And Configuration

Most read operations can work without a saved token when the target repository
is public. Authentication is required for write operations and for some
user-specific flows. Private repositories and some human-name fallback lookups
may still require authentication.

Commands that require authentication:

- `auth login`
- `issue comment`
- `pr comment`
- `pr create`
- `pr status`

Runtime token resolution order:

1. `GITEE_TOKEN`
2. saved config file token

Config directory resolution order:

1. `GITEE_CONFIG_DIR`
2. `XDG_CONFIG_HOME/gitee`
3. `HOME/.config/gitee`
4. current directory `./.gitee`

By default the saved token lives at `~/.config/gitee/config.toml`.

<details>
<summary>Relevant environment variables</summary>

- `GITEE_TOKEN`: overrides the saved token at runtime
- `GITEE_CONFIG_DIR`: points directly to the config directory
- `XDG_CONFIG_HOME`: used when `GITEE_CONFIG_DIR` is not set
- `HOME`: used for the default config path
- `GITEE_BASE_URL`: overrides the API base URL, which defaults to
  `https://gitee.com/api`; mainly useful for tests or local API mocking

</details>

## Automation Contracts

`gitee-cli` is designed to be scriptable:

- successful output goes to `stdout`
- errors go to `stderr`
- core commands support `--json`
- exit codes are stable enough to branch on in automation

Exit codes:

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
