# Repository Guidelines

## Project Structure & Module Organization
`gitee-cli` is a small Rust CLI. Keep executable startup thin in `src/main.rs`; route argument handling through `src/cli.rs`; and place feature logic in focused modules such as `src/auth.rs`, `src/repo.rs`, `src/config.rs`, `src/gitee_api.rs`, and `src/repo_context.rs`. Shared exports live in `src/lib.rs`. Put end-to-end CLI coverage in `tests/`, which currently contains `auth_cli.rs` and `repo_view_cli.rs`.

## Build, Test, and Development Commands
Use the Rust toolchain pinned in [`.tool-versions`](./.tool-versions) (`rust 1.94.0`).

- `cargo build` compiles the CLI.
- `cargo run -- auth status --json` runs a local command against the binary.
- `cargo run -- repo view --repo octo/demo --json` is a useful smoke test for command wiring.
- `cargo test` runs unit and integration tests.
- `cargo fmt -- --check` verifies formatting before review.
- `cargo clippy --all-targets --all-features -- -D warnings` is the preferred lint pass for new changes.

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, `snake_case` for modules/functions, `PascalCase` for types, and small enums/structs with explicit names. Prefer thin command parsing plus service-style methods, matching the current `AuthService` and `RepoService` split. Keep text and JSON output paths stable; changes to flags, stdout, stderr, or exit codes should be deliberate and test-backed.

## Testing Guidelines
Favor integration tests that exercise the compiled binary with `assert_cmd`. Mock Gitee HTTP calls with `httpmock`, isolate filesystem state with `tempfile`, and cover both text and `--json` output when behavior changes. Name integration tests by feature and surface, using the existing `*_cli.rs` pattern. Small pure helpers can keep unit tests inline under `#[cfg(test)]`.

## Commit & Pull Request Guidelines
Recent history mixes conventional prefixes and imperative subjects, for example `feat: add auth CLI skeleton` and `Add repo view command and repository context (#13)`. Prefer a short imperative subject, optionally prefixed with `feat:` or `chore:`, and include the issue or PR number when relevant. GitHub pull requests and issues should be written in English. PRs should describe the user-visible CLI change, list validation commands run, and include sample output when flags, JSON payloads, or exit codes change.

## Security & Configuration Tips
Never commit real Gitee tokens. Runtime configuration is read from `GITEE_TOKEN`, `GITEE_CONFIG_DIR`, `XDG_CONFIG_HOME`, and `HOME`; persisted credentials default to `~/.config/gitee-cli/config.toml`. Use `GITEE_BASE_URL` only for tests or local API mocking.
