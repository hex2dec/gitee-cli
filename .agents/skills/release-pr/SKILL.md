---
name: release-pr
description: Prepare a version release pull request for this repository by validating a target semver, bumping Cargo package metadata, running pre-release checks, and creating a GitHub PR with gh. Use when asked to prepare a release PR, bump the crate version, run release readiness checks, or open the GitHub PR for a new gitee-cli release.
---

# Release PR

Prepare a release branch and GitHub pull request for this Rust CLI repository.
Stop after the PR is open. Do not create a tag or publish a GitHub release unless the user explicitly asks.

## Input

- Target version in semver form, for example `0.2.0`
- Optional base branch; default to `main`

If the version is missing, ask for it before making changes.

## Preconditions

- Run from the repository root.
- Check `git status --short`. If the worktree is not clean, stop and ask the user how to proceed.
- Confirm `origin` points to the GitHub repository.
- Confirm GitHub CLI authentication with `gh auth status`.
- Prefer `git fetch origin --prune` before creating the release branch.

If any prerequisite is missing or fails, stop and explain the blocker before editing files.

## Guardrails

- Read the current version from `Cargo.toml`.
- Accept only a valid semver target version.
- Require the target version to differ from the current version.
- Check whether `release/v<version>` already exists locally or remotely. If it exists, stop and ask.
- Check whether tag `v<version>` already exists. If it exists, stop and ask.
- Check whether an open GitHub PR already exists for `release/v<version>` or title `release: v<version>`. If it exists, stop and ask.
- Do not edit `CHANGELOG.md`; this repository keeps release notes on the GitHub releases page.

## Version Update

Update only the release metadata:

- `Cargo.toml`: set `[package].version` to the target version
- `Cargo.lock`: allow the root `gitee-cli` package version to change if Cargo updates it as part of the version bump

Do not make unrelated dependency or source changes as part of this workflow.

## Validation

Run the standard release checks after the version bump:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If any command fails, stop and report the failure. Do not push a branch or create a PR with failing checks unless the user explicitly asks.

## Branch, Diff, and Commit

- Create branch `release/v<version>` from the chosen base branch. Default to `origin/main` when no base branch is provided.
- Review `git diff --stat` and `git diff`.
- Keep the diff limited to the version bump in `Cargo.toml` and, when applicable, the matching root package version change in `Cargo.lock`.
- Commit with this exact message: `release: v<version>`

## Pull Request

Push the branch and create the PR non-interactively with `gh pr create`.

Use:

- Branch: `release/v<version>`
- Title: `release: v<version>`
- Base branch: the selected base branch, default `main`

Use a PR body with these sections:

- `Summary`: `Prepare gitee-cli v<version> for release.`
- `Validation`: list the commands you ran
- `Release Plan`: state that merging the PR should be followed by creating tag `v<version>`, and that `.github/workflows/release.yml` will draft the GitHub release assets

Prefer `--body-file` when creating a multiline PR body.
