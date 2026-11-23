# Moonton Codex Desktop — 会话地图

> 由 scaffold_gen 生成。手动修改将在重新生成时被覆盖。

## Session Start Checklist

1. 阅读本文件了解项目概览
2. 检查 `.Codex/progress/session-state.json` 了解当前进度
3. 查询 `graph.json` 或 MCP 预定位任务相关节点与社区
4. 阅读分配的任务文件
5. 确认工作区就绪状态
6. 按 Plan-Build-Verify-Fix 流程执行

## Operating Model

- 单一事实源: [.Codex/project.json](.Codex/project.json)
- Flow: `/plan` -> `/build` -> `/verify` -> `/fix`
- Change flow: `/amend`
- 项目类型: desktop application
- 主语言: TypeScript/Rust
- 架构风格: layered
- 平台: windows, macos




## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Workspace Readiness

- 就绪度目标: `execution_ready`
- 当前状态: `execution_ready`
- Sprint 行为由就绪度决定
- 就绪度阻塞:
  _(none)_

## Rules

- `.Codex/rules/architecture.md`
- `.Codex/rules/completion-status.md`
- `.Codex/rules/decision-protocol.md`
- `.Codex/rules/platform-isolation.md`
- `.Codex/rules/project-constraints.md`
- `.Codex/rules/testing-gates.md`
- `.Codex/rules/thread-safety.md`
- `.Codex/rules/typescript-rust-safety.md`

## Module Ownership

| Module | Owner | Paths | Wiki | Summary |
|--------|-------|-------|------|---------|
| Codex-apps | core-Codex-apps | `src/components/claudeApps/`, `src/components/DynamicFunctionCreator.vue`, `src/components/DynamicFunctionHost.vue`, `src/stores/dynamicFunctionStore.ts`, `src/composables/claudeApps/`, `src-tauri/src/dynamic_function.rs` | [wiki](.Codex/wiki/modules/Codex-apps.md) | Lightweight app (dynamic function) system for creating, managing and running user-defined mini-apps with optional backend support and conversation history |
| Codex-bot | core-Codex-bot | `src/components/claudeBot/`, `src/stores/botStore.ts`, `src/composables/claudeBot/`, `src-tauri/src/claude_bot.rs`, `src-tauri/src/channels/`, `src-tauri/src/scheduler_heartbeat.rs` | [wiki](.Codex/wiki/modules/Codex-bot.md) | Autonomous Codex Bot engine with multi-channel integration (Discord, Telegram, Feishu) and task scheduling |
| coworks | core-coworks | `src/components/coworks/`, `src/components/Coworks.vue`, `src/components/CoworksModelSettings.vue`, `src/stores/devtoolsStore.ts`, `src/composables/coworks/`, `src-tauri/src/coworks_config.rs`, `src-tauri/src/coworks_fs.rs`, `src-tauri/src/coworks_sidecar.rs`, `sidecar/coworks/` | [wiki](.Codex/wiki/modules/coworks.md) | AI-powered collaboration hub with three modes: Chat (conversation), Code (development with file tree), and Cowork (full collaboration with Codex Agent SDK) |
| desktop | core-desktop | `src/components/Desktop.vue`, `src/components/WindowManager.vue`, `src/components/Window.vue`, `src/components/Taskbar.vue`, `src/components/DesktopIcon.vue`, `src/components/WidgetPanel.vue`, `src/components/FileManager.vue`, `src/stores/desktopStore.ts`, `src/stores/windowStore.ts`, `src/stores/fileSystemStore.ts` | [wiki](.Codex/wiki/modules/desktop.md) | Virtual desktop environment with icon management, multi-window system, taskbar, widget panel and virtual file manager |
| devtools | core-devtools | `src/components/devtools/`, `src/components/DevTools.vue`, `src/stores/devtoolsStore.ts`, `src-tauri/src/devtools.rs` | [wiki](.Codex/wiki/modules/devtools.md) | Developer tools for inspecting Cowork sessions, agent hierarchies, tool invocations, and performance metrics |
| llm-router | core-llm-router | `src/components/LlmRouter.vue`, `src-tauri/src/llm_router.rs` | [wiki](.Codex/wiki/modules/llm-router.md) | Codex Router (CCR) for configuring LLM model routing, providers, and API endpoints |
| marketplace | core-marketplace | `src/components/market/`, `src/components/PluginMarket.vue`, `src/components/PluginMarketFull.vue`, `src/stores/marketplaceStore.ts`, `src/stores/pluginStore.ts`, `src-tauri/src/marketplace.rs`, `src-tauri/src/skills.rs` | [wiki](.Codex/wiki/modules/marketplace.md) | Plugin marketplace for discovering, installing and managing Codex extensions (skills, agents, commands, MCP servers) |
| mcp | core-mcp | `src/components/McpManager.vue`, `src-tauri/src/mcp.rs` | [wiki](.Codex/wiki/modules/mcp.md) | Model Context Protocol (MCP) server configuration and management, supporting global and project-level servers |
| settings | core-settings | `src/components/Settings.vue`, `src/components/ClaudeAliases.vue`, `src/stores/settingsStore.ts`, `src-tauri/src/window_state.rs`, `src-tauri/src/app_paths.rs` | [wiki](.Codex/wiki/modules/settings.md) | Application settings management including theme, environment variables, Codex aliases, window state persistence, and data directory paths |
| system | core-system | `src-tauri/src/nodejs.rs`, `src-tauri/src/git.rs`, `src-tauri/src/uv.rs`, `src-tauri/src/system_aliases.rs`, `src-tauri/src/updater.rs`, `src/components/SetupGuide.vue`, `src/components/SetupGuideWindow.vue`, `src/components/UpdateModal.vue`, `src/composables/useSetupGuide.ts`, `src/composables/useAutoUpdate.ts` | [wiki](.Codex/wiki/modules/system.md) | System dependency management (Node.js, Git, UV), auto-update mechanism, first-run setup guide, and system alias resolution |
| terminal | core-terminal | `src/components/Terminal.vue`, `src/components/TerminalFilePanel.vue`, `src/components/TerminalSkillPanel.vue`, `src/stores/terminalStore.ts`, `src/composables/useTerminalPty.ts`, `src-tauri/src/pty.rs`, `src-tauri/src/node_pty.rs` | [wiki](.Codex/wiki/modules/terminal.md) | Cross-platform pseudo-terminal (PTY) with xterm.js UI, supporting both Unix portable-pty and Windows ConPTY |

## Commands

- [.Codex/commands/plan.md](.Codex/commands/plan.md) — 规划任务实现
- [.Codex/commands/build.md](.Codex/commands/build.md)
- [.Codex/commands/verify.md](.Codex/commands/verify.md)
- [.Codex/commands/fix.md](.Codex/commands/fix.md)
- [.Codex/commands/sprint.md](.Codex/commands/sprint.md) — 波次调度并行执行
- [.Codex/commands/sprint-review.md](.Codex/commands/sprint-review.md)
- [.Codex/commands/sync-progress.md](.Codex/commands/sync-progress.md)
- [.Codex/commands/amend.md](.Codex/commands/amend.md) — 修改任务范围
- [.Codex/commands/quick.md](.Codex/commands/quick.md) — 快速单任务执行
- [.Codex/commands/debug.md](.Codex/commands/debug.md) — 调试辅助
- [.Codex/commands/wikiorganize.md](.Codex/commands/wikiorganize.md) — 重整 wiki 文档

## Testing

- 验证命令:
  - `npm run typecheck`
  - `npm run lint`
  - `npm run test`
- 引导检查:
  _(none)_
- 覆盖率追踪: [.Codex/tdd/index.md](.Codex/tdd/index.md)

## Agent Skills (internal)

- [.Codex/agent-skills/read-task.md](.Codex/agent-skills/read-task.md)
- [.Codex/agent-skills/run-verify.md](.Codex/agent-skills/run-verify.md)
- [.Codex/agent-skills/write-session-log.md](.Codex/agent-skills/write-session-log.md)
- [.Codex/agent-skills/load-community-context.md](.Codex/agent-skills/load-community-context.md)

## Agents

- [.Codex/agents/lead.md](.Codex/agents/lead.md) — 编排
- [.Codex/agents/core-Codex-apps.md](.Codex/agents/core-Codex-apps.md) — Codex-apps
- [.Codex/agents/core-Codex-bot.md](.Codex/agents/core-Codex-bot.md) — Codex-bot
- [.Codex/agents/core-coworks.md](.Codex/agents/core-coworks.md) — coworks
- [.Codex/agents/core-desktop.md](.Codex/agents/core-desktop.md) — desktop
- [.Codex/agents/core-devtools.md](.Codex/agents/core-devtools.md) — devtools
- [.Codex/agents/core-llm-router.md](.Codex/agents/core-llm-router.md) — llm-router
- [.Codex/agents/core-marketplace.md](.Codex/agents/core-marketplace.md) — marketplace
- [.Codex/agents/core-mcp.md](.Codex/agents/core-mcp.md) — mcp
- [.Codex/agents/core-settings.md](.Codex/agents/core-settings.md) — settings
- [.Codex/agents/core-system.md](.Codex/agents/core-system.md) — system
- [.Codex/agents/core-terminal.md](.Codex/agents/core-terminal.md) — terminal
- [.Codex/agents/review.md](.Codex/agents/review.md) — 审查
- [.Codex/agents/gc.md](.Codex/agents/gc.md) — 清理

## Knowledge Base

- [Wiki Index](.Codex/wiki/index.md) — 领域知识库
- [TDD Strategy](.Codex/tdd/index.md) — 测试规格和骨架
- [Architecture](.Codex/wiki/architecture/overview.md)
- [API Contracts](.Codex/wiki/specs/api-contracts.md)
- [.Codex/wiki/GRAPH_REPORT.md](.Codex/wiki/GRAPH_REPORT.md) — god 节点 / 意外连接 / 建议问题
- [.Codex/wiki/modules/Codex-apps.md](.Codex/wiki/modules/Codex-apps.md)
- [.Codex/wiki/modules/Codex-bot.md](.Codex/wiki/modules/Codex-bot.md)
- [.Codex/wiki/modules/coworks.md](.Codex/wiki/modules/coworks.md)
- [.Codex/wiki/modules/desktop.md](.Codex/wiki/modules/desktop.md)
- [.Codex/wiki/modules/devtools.md](.Codex/wiki/modules/devtools.md)
- [.Codex/wiki/modules/llm-router.md](.Codex/wiki/modules/llm-router.md)
- [.Codex/wiki/modules/marketplace.md](.Codex/wiki/modules/marketplace.md)
- [.Codex/wiki/modules/mcp.md](.Codex/wiki/modules/mcp.md)
- [.Codex/wiki/modules/settings.md](.Codex/wiki/modules/settings.md)
- [.Codex/wiki/modules/system.md](.Codex/wiki/modules/system.md)
- [.Codex/wiki/modules/terminal.md](.Codex/wiki/modules/terminal.md)

## Milestones

- [Maintenance Foundation](.Codex/milestones/m1.md) (6 tasks)

## Notes

- Tauri v2 IPC bridge between Vue 3 frontend and Rust backend
- Sidecar processes: Coworks SDK (Codex Agent SDK), Feishu WebSocket listener
- Plugin marketplace with Git-based distribution
- Test infrastructure in place: Vitest (frontend), Cargo test (Rust), 93.6% aggregate coverage on 8 targeted modules
- Knowledge graph hook installed; auto-rebuild on commit available via graphify hooks
- All M1 Foundation tasks completed
- Knowledge graph hook 已安装在 .Codex/settings.json；它会在执行文件搜索前提醒 Codex 先查阅 GRAPH_REPORT.md。
- 首次建议运行 `python scripts/install_graphify_hooks.py --install` 开启 commit 自动重建
- 所有里程碑完成后，建议执行 `/wikiorganize` 整理 wiki 文档
