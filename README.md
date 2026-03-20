# gitee-cli

`gitee-cli` is a small Rust command-line tool for authenticating with Gitee and inspecting repository metadata.

## Quick Start

The examples below assume the installed executable is available on your `PATH` as `gitee`.

### Authentication

Check the current authentication state:

```bash
gitee auth status --json
```

Save a token after validating it with Gitee:

```bash
gitee auth login --token "$GITEE_TOKEN" --json
```

Read a token from stdin instead:

```bash
printf '%s\n' "$TOKEN" | gitee auth login --with-token --json
```

Clear the saved token:

```bash
gitee auth logout --json
```

### Repository View

Inspect a repository by explicit slug:

```bash
gitee repo view --repo octo/demo --json
```

Inside a local Gitee repository, infer the slug from `origin`:

```bash
gitee repo view --json
```

## Development

The project uses Rust `1.94.0` from [`.tool-versions`](./.tool-versions).

```bash
cargo build
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Configuration

- `GITEE_TOKEN`: overrides the saved token at runtime
- `GITEE_CONFIG_DIR`: sets the config directory directly
- `GITEE_BASE_URL`: useful for tests or local API mocking

## License

MIT. See [LICENSE](./LICENSE).
