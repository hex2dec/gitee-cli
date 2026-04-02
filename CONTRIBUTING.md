# Contributing to gitee-cli

Thanks for contributing to `gitee-cli`.

This document covers local development setup, validation commands, repository
conventions, and pull request expectations. User-facing install and workflow
examples stay in [README.md](./README.md).

## Development Environment

The project uses Rust `1.94.0` from [`.tool-versions`](./.tool-versions).

Common local commands:

```bash
cargo build
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Useful smoke tests while wiring commands:

```bash
cargo run -- auth status --json
cargo run -- repo view --repo octo/demo --json
cargo run -- issue list --repo octo/demo --json
cargo run -- pr list --repo octo/demo --json
```

## Project Layout

Keep executable startup thin in `src/main.rs`, route argument parsing through
`src/cli.rs`, and keep feature logic in focused modules such as:

- `src/auth.rs`
- `src/repo.rs`
- `src/issue.rs`
- `src/pr.rs`
- `src/config.rs`
- `src/gitee_api.rs`
- `src/repo_context.rs`
- `src/command.rs`
- `src/lib.rs`

End-to-end CLI coverage lives in `tests/`, including:

- `tests/auth_cli.rs`
- `tests/repo_view_cli.rs`
- `tests/repo_clone_cli.rs`
- `tests/issue_cli.rs`
- `tests/pr_view_cli.rs`
- `tests/pr_list_cli.rs`
- `tests/pr_create_cli.rs`
- `tests/pr_comment_cli.rs`
- `tests/pr_status_cli.rs`
- `tests/pr_checkout_cli.rs`

## Coding and Testing Guidelines

Follow standard Rust conventions:

- 4-space indentation
- `snake_case` for modules and functions
- `PascalCase` for types
- thin command parsing with service-style logic

Keep CLI text output, `--json` payloads, flags, stderr messages, and exit codes
stable unless a deliberate behavior change is covered by tests and documented in
the pull request.

When adding or changing tests:

- prefer integration tests with `assert_cmd`
- use `httpmock` for Gitee HTTP calls
- use `tempfile` for filesystem isolation
- cover both text and `--json` output when behavior changes
- keep using real local git fixtures for git-dependent flows such as repo
  context inference, clone, PR create, PR status, and PR checkout

## Pull Requests

Use a short imperative commit subject. A `feat:` or `chore:` prefix is fine, but
do not append PR numbers unless the change specifically requires that format.

Pull requests and issues should be written in English. A good PR includes:

- the user-visible CLI change
- the validation commands you ran
- sample output when flags, JSON payloads, or exit codes changed

If a user-facing documentation change affects both languages, update
[README.md](./README.md) and [README_CN.md](./README_CN.md) together. Keep
English and Chinese contribution guidance aligned in
[CONTRIBUTING.md](./CONTRIBUTING.md) and [CONTRIBUTING_CN.md](./CONTRIBUTING_CN.md).

## Security and Configuration

Never commit real Gitee tokens.

Runtime configuration may read from:

- `GITEE_TOKEN`
- `GITEE_CONFIG_DIR`
- `XDG_CONFIG_HOME`
- `HOME`
- `USERPROFILE`
- `HOMEDRIVE`
- `HOMEPATH`

Persisted credentials default to `~/.config/gitee/config.toml` when
`GITEE_CONFIG_DIR` is not set. Use `GITEE_BASE_URL` only for tests or local API
mocking.
