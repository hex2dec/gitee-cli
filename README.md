# gitee-cli

`gitee-cli` is a small Rust command-line tool for authenticating with Gitee and inspecting repository metadata.

## Current Commands

### Authentication

Check the current authentication state:

```bash
cargo run -- auth status --json
```

Save a token after validating it with Gitee:

```bash
cargo run -- auth login --token "$GITEE_TOKEN" --json
```

Read a token from stdin instead:

```bash
printf '%s\n' "$TOKEN" | cargo run -- auth login --with-token --json
```

Clear the saved token:

```bash
cargo run -- auth logout --json
```

### Repository View

Inspect a repository by explicit slug:

```bash
cargo run -- repo view --repo octo/demo --json
```

Inside a local Gitee repository, infer the slug from `origin`:

```bash
cargo run -- repo view --json
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
- `XDG_CONFIG_HOME`: used when `GITEE_CONFIG_DIR` is unset
- `GITEE_BASE_URL`: useful for tests or local API mocking

Saved credentials default to `~/.config/gitee-cli/config.toml`.

## License

MIT. See [LICENSE](./LICENSE).
