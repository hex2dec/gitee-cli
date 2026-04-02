# 参与贡献

感谢你为 `gitee-cli` 做出贡献。

本文档覆盖本地开发环境、验证命令、仓库约定和 Pull Request 预期。面向用户
的安装与工作流示例保留在 [README_CN.md](./README_CN.md) 中。

## 开发环境

项目使用 [`.tool-versions`](./.tool-versions) 中指定的 Rust `1.94.0`。

本地常用命令：

```bash
cargo build
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

命令接线时常用的 smoke tests：

```bash
cargo run -- auth status --json
cargo run -- repo view --repo octo/demo --json
cargo run -- issue list --repo octo/demo --json
cargo run -- pr list --repo octo/demo --json
```

## 项目结构

保持 `src/main.rs` 中的启动逻辑足够薄，把参数解析集中在 `src/cli.rs`，并把
功能逻辑放在职责明确的模块中，例如：

- `src/auth.rs`
- `src/repo.rs`
- `src/issue.rs`
- `src/pr.rs`
- `src/config.rs`
- `src/gitee_api.rs`
- `src/repo_context.rs`
- `src/command.rs`
- `src/lib.rs`

端到端 CLI 测试位于 `tests/`，当前包括：

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

## 编码与测试约定

遵循标准 Rust 风格：

- 4 空格缩进
- 模块和函数使用 `snake_case`
- 类型使用 `PascalCase`
- 命令解析保持精简，核心逻辑尽量采用 service 风格组织

CLI 的文本输出、`--json` 结果、参数、stderr 信息和退出码应尽量保持稳定。
如果需要有意调整行为，应补充测试，并在 Pull Request 中说明。

新增或修改测试时：

- 优先使用 `assert_cmd` 编写集成测试
- 使用 `httpmock` 模拟 Gitee HTTP 调用
- 使用 `tempfile` 隔离文件系统状态
- 行为变化时同时覆盖文本输出和 `--json` 输出
- 对 repo context 推断、clone、PR create、PR status、PR checkout 等依赖
  git 的流程，继续使用本地真实 git fixture，而不是改成 mock

## Pull Request 约定

commit subject 建议使用简短的祈使句；可以带 `feat:` 或 `chore:` 前缀，但除
非明确需要，不要在手写 commit subject 里追加 PR 编号。

GitHub 上的 Pull Request 和 Issue 请使用英文。一个合格的 PR 应包含：

- 用户可见的 CLI 变化说明
- 实际执行过的验证命令
- 当参数、JSON 载荷或退出码发生变化时，附上示例输出

如果用户可见文档在中英文两边都有对应内容，请同时更新
[README.md](./README.md) 和 [README_CN.md](./README_CN.md)。贡献说明也应保持
[CONTRIBUTING.md](./CONTRIBUTING.md) 与
[CONTRIBUTING_CN.md](./CONTRIBUTING_CN.md) 同步。

## 安全与配置

不要提交真实的 Gitee token。

运行时配置可能会读取以下环境变量：

- `GITEE_TOKEN`
- `GITEE_CONFIG_DIR`
- `XDG_CONFIG_HOME`
- `HOME`
- `USERPROFILE`
- `HOMEDRIVE`
- `HOMEPATH`

未设置 `GITEE_CONFIG_DIR` 时，持久化凭据默认保存在
`~/.config/gitee/config.toml`。`GITEE_BASE_URL` 只应用于测试或本地 API
mock。
