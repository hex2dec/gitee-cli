# Repository Guidelines

## Project Structure & Module Organization
`gitee-cli` is a small Rust CLI. Keep executable startup thin in `src/main.rs`; route argument handling through `src/cli.rs`; and place feature logic in focused modules such as `src/auth.rs`, `src/repo.rs`, `src/issue.rs`, `src/pr.rs`, `src/config.rs`, `src/gitee_api.rs`, and `src/repo_context.rs`. Shared exports live in `src/lib.rs`, and command outcome / exit code plumbing lives in `src/command.rs`. Put end-to-end CLI coverage in `tests/`, which currently includes `auth_cli.rs`, `repo_view_cli.rs`, `repo_clone_cli.rs`, `issue_cli.rs`, `pr_view_cli.rs`, `pr_list_cli.rs`, `pr_create_cli.rs`, `pr_comment_cli.rs`, `pr_status_cli.rs`, and `pr_checkout_cli.rs`.

## Build, Test, and Development Commands
Use the Rust toolchain pinned in [`.tool-versions`](./.tool-versions) (`rust 1.94.0`).

- `cargo build` compiles the CLI.
- `cargo run -- auth status --json` runs a local command against the binary.
- `cargo run -- repo view --repo octo/demo --json` is a useful smoke test for command wiring.
- `cargo run -- issue list --repo octo/demo --json` is a useful smoke test for issue command wiring.
- `cargo run -- pr list --repo octo/demo --json` is a useful smoke test for pull request command wiring.
- `cargo test` runs unit and integration tests.
- `cargo fmt -- --check` verifies formatting before review.
- `cargo clippy --all-targets --all-features -- -D warnings` is the preferred lint pass for new changes.

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, `snake_case` for modules/functions, `PascalCase` for types, and small enums/structs with explicit names. Prefer thin command parsing plus service-style methods, matching the current `AuthService`, `RepoService`, `IssueService`, and `PrService` split. Keep text and JSON output paths stable; changes to flags, stdout, stderr, or exit codes should be deliberate and test-backed.

## Testing Guidelines
Favor integration tests that exercise the compiled binary with `assert_cmd`. Mock Gitee HTTP calls with `httpmock`, isolate filesystem state with `tempfile`, and cover both text and `--json` output when behavior changes. Name integration tests by feature and surface, using the existing `*_cli.rs` pattern. For git-dependent flows such as repo context inference, clone, PR create, PR status, and PR checkout, keep using real local git fixtures rather than replacing those paths with mocks. Small pure helpers can keep unit tests inline under `#[cfg(test)]`.

## Commit & Pull Request Guidelines
Recent history includes both plain commit subjects such as `feat: add auth CLI skeleton` and historical subjects suffixed with `(#13)`. Treat the `(#13)` style as repository history, not as the default format for new handwritten commits. Prefer a short imperative subject, optionally prefixed with `feat:` or `chore:`, and do not append PR numbers to manual commit messages unless the user explicitly asks for that format. If an issue or PR reference matters, put it in the PR title, PR description, or linked metadata instead of the commit subject. GitHub pull requests and issues should be written in English. PRs should describe the user-visible CLI change, list validation commands run, and include sample output when flags, JSON payloads, or exit codes change.

## Documentation Guidelines
Keep `README.md` and `README_CN.md` in sync for user-facing behavior, install steps, command examples, release/platform notes, and workflow guidance. When one of those files changes, update the other in the same change unless there is a deliberate language-specific exception and you call it out explicitly.

## Security & Configuration Tips
Never commit real Gitee tokens. Runtime configuration is read from `GITEE_TOKEN`, `GITEE_CONFIG_DIR`, `XDG_CONFIG_HOME`, `HOME`, `USERPROFILE`, `HOMEDRIVE`, and `HOMEPATH`; persisted credentials default to `~/.config/gitee/config.toml` when `GITEE_CONFIG_DIR` is not set. Use `GITEE_BASE_URL` only for tests or local API mocking.
