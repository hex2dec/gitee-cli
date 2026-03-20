# 面向 Agent 的 gitee.com 工作流

中文文档对应当前英文版 README。若两者出现差异，请以
[README.md](./README.md) 为准。

`gitee-cli` 是一个面向 Agent 的命令行工具，用于在脚本、本地终端和
AI 驱动的工作流中操作 `gitee.com`。

它为认证、仓库检查、Issue 处理和 Pull Request 工作流提供了一组小而稳
定的命令接口，避免你直接拼接底层 Gitee API 请求。

安装后的可执行文件名为 `gitee`。

> `gitee-cli` 是一个非官方社区项目，与 Gitee 或 `gitee.com` 不存在关
> 联关系，也未获得其认可、背书或赞助。
>
> Gitee 和 `gitee.com` 是其各自权利人的商标或注册商标。本文档仅为说明
> 平台兼容性和适用范围而引用这些名称。

## 为什么做这个项目

`gitee-cli` 面向的是自动化和日常开发中最常见、最频繁的 Gitee 工作流：

- 在执行任务前先确认认证是否可用
- 通过仓库 slug 或本地 checkout 检查仓库元数据
- 在修改代码前先阅读 Issue 上下文
- 在终端里查看、列出、创建、评论和检出 Pull Request
- 为脚本提供稳定的 `--json` 输出和明确的退出码

这个项目是有明确取舍的：

- 目标平台是 `gitee.com`
- 优先支持显式、非交互式工作流
- 同时支持人类可读输出和稳定的 `--json`
- 在合适时使用本地 git 上下文来简化常见命令

## 适合谁使用

如果你希望获得下面这些能力，可以使用 `gitee-cli`：

- 一个适合 AI Agent 和自动化脚本的 Gitee 工作流工具
- 一个可以直接在终端里查看仓库、Issue 和 Pull Request 的工具
- 写操作通过参数、文件或 stdin 提供内容，而不是依赖交互式提示
- 行为稳定、便于脚本安全调用

## 从这里开始

当前项目通过源码构建。

本地构建二进制：

```bash
cargo build
```

在仓库内直接运行命令进行开发调试：

```bash
cargo run -- auth status --json
```

使用 token 登录：

```bash
gitee auth login --token "$GITEE_TOKEN" --json
```

检查认证是否可用：

```bash
gitee auth status --json
```

直接查看指定仓库：

```bash
gitee repo view --repo octo/demo --json
```

或者在本地 Gitee 仓库目录中，让 `gitee-cli` 通过 `origin` 自动推断目标
仓库：

```bash
gitee repo view --json
```

如果你只想先体验一条完整路径，建议按下面顺序试用：

1. 构建 CLI。
2. 使用个人访问令牌登录。
3. 运行 `gitee repo view --repo octo/demo --json`。
4. 进入一个本地 checkout 后，运行 `gitee pr status --state open --limit 10 --json`。

## 常见工作流

### 在开始工作前检查认证状态

当脚本或 Agent 需要在操作仓库或 API 之前尽早失败时，先执行
`auth status`：

```bash
gitee auth status --json
```

如果希望通过 stdin 保存 token，而不是通过参数传入：

```bash
printf '%s\n' "$TOKEN" | gitee auth login --with-token --json
```

### 快速检查仓库信息

已知仓库 slug 时：

```bash
gitee repo view --repo octo/demo --json
```

已经位于本地 checkout 内时：

```bash
gitee repo view --json
```

默认使用 HTTPS 克隆：

```bash
gitee repo clone octo/demo
```

使用 SSH 克隆到指定目录：

```bash
gitee repo clone octo/demo demo-ssh --ssh --json
```

### 在修改代码前先阅读 Issue 上下文

列出当前仓库的开放 Issue：

```bash
gitee issue list --state open --page 1 --per-page 20 --json
```

查看指定仓库中的单个 Issue：

```bash
gitee issue view I123 --repo octo/demo --json
```

需要查看历史讨论时，显式包含评论：

```bash
gitee issue view I123 --repo octo/demo --comments --page 1 --per-page 20 --json
```

以非交互方式发布一条跟进评论：

```bash
gitee issue comment I123 --repo octo/demo --body "Thanks for the report" --json
```

### 不离开终端处理 Pull Request

查看一个 Pull Request：

```bash
gitee pr view 42 --repo octo/demo --json
```

按条件列出 Pull Request：

```bash
gitee pr list --repo octo/demo --state open --author octocat --limit 10 --json
```

查看与当前分支或当前用户相关的 Pull Request：

```bash
gitee pr status --state open --limit 10 --json
```

基于当前分支创建 Pull Request：

```bash
gitee pr create --title "Use local head" --base develop --body "Built from the local branch"
```

从文件读取 PR 描述：

```bash
gitee pr create --repo octo/demo --head feature/body-file --title "Read body file" --body-file ./body.md --json
```

对 Pull Request 发表评论：

```bash
gitee pr comment 42 --repo octo/demo --body "Ship it" --json
```

将 Pull Request 的 head 分支检出到当前本地仓库：

```bash
gitee pr checkout 42 --repo octo/demo --json
```

## 本地仓库上下文

当省略 `--repo` 时，`gitee-cli` 会尝试从当前本地 git checkout 中推断目标
仓库。这会让你在正确仓库目录里执行常见命令时更简洁。

支持使用本地仓库上下文的命令包括：

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

`pr status` 总是要求当前目录是一个本地 git checkout。`pr checkout` 还要
求该仓库存在 `origin` 远程。

<details>
<summary>支持的 <code>origin</code> URL 形式</summary>

- `git@gitee.com:owner/repo.git`
- `ssh://git@gitee.com/owner/repo.git`
- `https://gitee.com/owner/repo.git`
- `http://gitee.com/owner/repo.git`

</details>

## 认证与配置

对于公开仓库，多数只读操作在没有保存 token 的情况下也可以工作。写操作
和部分与用户身份相关的流程要求认证。私有仓库，以及某些基于 human-name
回退解析的场景，也可能要求认证。

需要认证的命令包括：

- `auth login`
- `issue comment`
- `pr comment`
- `pr create`
- `pr status`

运行时 token 的解析优先级：

1. `GITEE_TOKEN`
2. 已保存到配置文件中的 token

配置目录的解析优先级：

1. `GITEE_CONFIG_DIR`
2. `XDG_CONFIG_HOME/gitee`
3. `HOME/.config/gitee`
4. 当前目录 `./.gitee`

默认情况下，保存的 token 位于 `~/.config/gitee/config.toml`。

<details>
<summary>相关环境变量</summary>

- `GITEE_TOKEN`：运行时覆盖已保存的 token
- `GITEE_CONFIG_DIR`：直接指定配置目录
- `XDG_CONFIG_HOME`：未设置 `GITEE_CONFIG_DIR` 时参与默认路径解析
- `HOME`：用于默认配置路径
- `GITEE_BASE_URL`：覆盖 API 基础地址，默认值为
  `https://gitee.com/api`；主要用于测试或本地 API mock

</details>

## 自动化契约

`gitee-cli` 的设计目标之一就是便于脚本调用：

- 成功输出写入 `stdout`
- 错误信息写入 `stderr`
- 核心命令支持 `--json`
- 退出码保持稳定，便于自动化流程分支判断

退出码：

- `0`：成功
- `2`：用法错误
- `3`：认证错误或需要认证
- `4`：配置错误
- `5`：远程请求错误
- `6`：资源不存在
- `7`：本地 git 错误

## 开发

项目使用 [`.tool-versions`](./.tool-versions) 中指定的 Rust `1.94.0`。

```bash
cargo build
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

常用 smoke tests：

```bash
cargo run -- auth status --json
cargo run -- repo view --repo octo/demo --json
cargo run -- issue list --repo octo/demo --json
cargo run -- pr list --repo octo/demo --json
```

## 许可证

MIT。见 [LICENSE](./LICENSE)。
