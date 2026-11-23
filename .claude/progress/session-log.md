# Moonton Claude Desktop 会话日志

> 由 scaffold_gen 生成。写入此文件请通过 `@write-session-log` skill，不要直接追加。硬限 150 行。

## 待办 (Pending)

<!-- 未完成 / 阻塞 / 下一步。永不压缩。 -->

_尚无待办。_

## 归档 (Archive)

_尚无已完成的里程碑。_

## 近期活动 (Recent Activity)

### [debug] 2026-05-27 — bundled nodejs PATH 抢占 + npm 全局安装路径不在 PATH 中

**根因**：bundled Node.js 的 `node` binary 在 `nodejs/node`（非标准 `nodejs/bin/node`），导致：
1. npm 默认 prefix `path.resolve(execPath, '..', '..')` 算成 `~/.MoontonClaudeDesktop`，`npm install -g` 落到 `~/.MoontonClaudeDesktop/bin/`（不在 PATH 中）
2. macOS `~/.zshrc` 用 `export PATH="bundled:$PATH"` 强 prepend，永久抢占任何后装的系统 nodejs

**方案权衡**：
- macOS 选 Option B：bundled 改为 append（`export PATH="$PATH:bundled"`），系统 nodejs 优先，bundled 仅作 `claude` shebang fallback
- Windows 维持现状：注册表写入是用户 PATH 且 push（append），系统 PATH > 用户 PATH 自然保证官方 MSI 安装的 nodejs 优先；只有 nvm-windows 等用户级工具用户可能受影响（边缘场景，可接受）

**修复**：
- `pty.rs` / `node_pty.rs`：PTY 注入 `NPM_CONFIG_PREFIX`（macOS=`~/.npm-global`，Windows=`%APPDATA%\npm`）
- `nodejs.rs` `add_to_shell_profile`：写 `NPM_CONFIG_PREFIX`、npm-global/bin 走 prepend、bundled nodejs 走 append；NPM_CONFIG_PREFIX 幂等检查改为完整 `export NPM_CONFIG_PREFIX=` 字符串匹配

**已知局限**：
- 老用户 `~/.zshrc` 残留旧的 prepend 条目不会自动迁移；新代码幂等检查会跳过重复写入
- Windows nvm-windows 用户若在装 app 之后才装 nvm，bundled 仍可能优先

**验证**：`cargo check` 通过。

## 当前里程碑 (Current Milestone)

_尚无记录。_
