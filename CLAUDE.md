# Moonton Claude Desktop — 会话地图

> 由 scaffold_gen 生成。手动修改将在重新生成时被覆盖。

## Session Start Checklist

1. 阅读本文件了解项目概览
2. 检查 `.claude/progress/session-state.json` 了解当前进度
3. 查询 `graph.json` 或 MCP 预定位任务相关节点与社区
4. 阅读分配的任务文件
5. 确认工作区就绪状态
6. 按 Plan-Build-Verify-Fix 流程执行

## Operating Model

- 单一事实源: [.claude/project.json](.claude/project.json)
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

- `.claude/rules/architecture.md`
- `.claude/rules/completion-status.md`
- `.claude/rules/decision-protocol.md`
- `.claude/rules/platform-isolation.md`
- `.claude/rules/project-constraints.md`
- `.claude/rules/testing-gates.md`
- `.claude/rules/thread-safety.md`
- `.claude/rules/typescript-rust-safety.md`

## Module Ownership

| Module | Owner | Paths | Wiki | Summary |
|--------|-------|-------|------|---------|
| claude-apps | core-claude-apps | `src/components/claudeApps/`, `src/components/DynamicFunctionCreator.vue`, `src/components/DynamicFunctionHost.vue`, `src/stores/dynamicFunctionStore.ts`, `src/composables/claudeApps/`, `src-tauri/src/dynamic_function.rs` | [wiki](.claude/wiki/modules/claude-apps.md) | Lightweight app (dynamic function) system for creating, managing and running user-defined mini-apps with optional backend support and conversation history |
| claude-bot | core-claude-bot | `src/components/claudeBot/`, `src/stores/botStore.ts`, `src/composables/claudeBot/`, `src-tauri/src/claude_bot.rs`, `src-tauri/src/channels/`, `src-tauri/src/scheduler_heartbeat.rs` | [wiki](.claude/wiki/modules/claude-bot.md) | Autonomous Claude Bot engine with multi-channel integration (Discord, Telegram, Feishu) and task scheduling |
| coworks | core-coworks | `src/components/coworks/`, `src/components/Coworks.vue`, `src/components/CoworksModelSettings.vue`, `src/stores/devtoolsStore.ts`, `src/composables/coworks/`, `src-tauri/src/coworks_config.rs`, `src-tauri/src/coworks_fs.rs`, `src-tauri/src/coworks_sidecar.rs`, `sidecar/coworks/` | [wiki](.claude/wiki/modules/coworks.md) | AI-powered collaboration hub with three modes: Chat (conversation), Code (development with file tree), and Cowork (full collaboration with Claude Agent SDK) |
| desktop | core-desktop | `src/components/Desktop.vue`, `src/components/WindowManager.vue`, `src/components/Window.vue`, `src/components/Taskbar.vue`, `src/components/DesktopIcon.vue`, `src/components/WidgetPanel.vue`, `src/components/FileManager.vue`, `src/stores/desktopStore.ts`, `src/stores/windowStore.ts`, `src/stores/fileSystemStore.ts` | [wiki](.claude/wiki/modules/desktop.md) | Virtual desktop environment with icon management, multi-window system, taskbar, widget panel and virtual file manager |
| devtools | core-devtools | `src/components/devtools/`, `src/components/DevTools.vue`, `src/stores/devtoolsStore.ts`, `src-tauri/src/devtools.rs` | [wiki](.claude/wiki/modules/devtools.md) | Developer tools for inspecting Cowork sessions, agent hierarchies, tool invocations, and performance metrics |
| llm-router | core-llm-router | `src/components/LlmRouter.vue`, `src-tauri/src/llm_router.rs` | [wiki](.claude/wiki/modules/llm-router.md) | Claude Code Router (CCR) for configuring LLM model routing, providers, and API endpoints |
| marketplace | core-marketplace | `src/components/market/`, `src/components/PluginMarket.vue`, `src/components/PluginMarketFull.vue`, `src/stores/marketplaceStore.ts`, `src/stores/pluginStore.ts`, `src-tauri/src/marketplace.rs`, `src-tauri/src/skills.rs` | [wiki](.claude/wiki/modules/marketplace.md) | Plugin marketplace for discovering, installing and managing Claude Code extensions (skills, agents, commands, MCP servers) |
| mcp | core-mcp | `src/components/McpManager.vue`, `src-tauri/src/mcp.rs` | [wiki](.claude/wiki/modules/mcp.md) | Model Context Protocol (MCP) server configuration and management, supporting global and project-level servers |
| settings | core-settings | `src/components/Settings.vue`, `src/components/ClaudeAliases.vue`, `src/stores/settingsStore.ts`, `src-tauri/src/window_state.rs`, `src-tauri/src/app_paths.rs` | [wiki](.claude/wiki/modules/settings.md) | Application settings management including theme, environment variables, Claude aliases, window state persistence, and data directory paths |
| system | core-system | `src-tauri/src/nodejs.rs`, `src-tauri/src/git.rs`, `src-tauri/src/uv.rs`, `src-tauri/src/system_aliases.rs`, `src-tauri/src/updater.rs`, `src/components/SetupGuide.vue`, `src/components/SetupGuideWindow.vue`, `src/components/UpdateModal.vue`, `src/composables/useSetupGuide.ts`, `src/composables/useAutoUpdate.ts` | [wiki](.claude/wiki/modules/system.md) | System dependency management (Node.js, Git, UV), auto-update mechanism, first-run setup guide, and system alias resolution |
| terminal | core-terminal | `src/components/Terminal.vue`, `src/components/TerminalFilePanel.vue`, `src/components/TerminalSkillPanel.vue`, `src/stores/terminalStore.ts`, `src/composables/useTerminalPty.ts`, `src-tauri/src/pty.rs`, `src-tauri/src/node_pty.rs` | [wiki](.claude/wiki/modules/terminal.md) | Cross-platform pseudo-terminal (PTY) with xterm.js UI, supporting both Unix portable-pty and Windows ConPTY |

## Commands

- [.claude/commands/plan.md](.claude/commands/plan.md) — 规划任务实现
- [.claude/commands/build.md](.claude/commands/build.md)
- [.claude/commands/verify.md](.claude/commands/verify.md)
- [.claude/commands/fix.md](.claude/commands/fix.md)
- [.claude/commands/sprint.md](.claude/commands/sprint.md) — 波次调度并行执行
- [.claude/commands/sprint-review.md](.claude/commands/sprint-review.md)
- [.claude/commands/sync-progress.md](.claude/commands/sync-progress.md)
- [.claude/commands/amend.md](.claude/commands/amend.md) — 修改任务范围
- [.claude/commands/quick.md](.claude/commands/quick.md) — 快速单任务执行
- [.claude/commands/debug.md](.claude/commands/debug.md) — 调试辅助
- [.claude/commands/wikiorganize.md](.claude/commands/wikiorganize.md) — 重整 wiki 文档

## Testing

- 验证命令:
  - `npm run typecheck`
  - `npm run lint`
  - `npm run test`
- 引导检查:
  _(none)_
- 覆盖率追踪: [.claude/tdd/index.md](.claude/tdd/index.md)

## Agent Skills (internal)

- [.claude/agent-skills/read-task.md](.claude/agent-skills/read-task.md)
- [.claude/agent-skills/run-verify.md](.claude/agent-skills/run-verify.md)
- [.claude/agent-skills/write-session-log.md](.claude/agent-skills/write-session-log.md)
- [.claude/agent-skills/load-community-context.md](.claude/agent-skills/load-community-context.md)

## Agents

- [.claude/agents/lead.md](.claude/agents/lead.md) — 编排
- [.claude/agents/core-claude-apps.md](.claude/agents/core-claude-apps.md) — claude-apps
- [.claude/agents/core-claude-bot.md](.claude/agents/core-claude-bot.md) — claude-bot
- [.claude/agents/core-coworks.md](.claude/agents/core-coworks.md) — coworks
- [.claude/agents/core-desktop.md](.claude/agents/core-desktop.md) — desktop
- [.claude/agents/core-devtools.md](.claude/agents/core-devtools.md) — devtools
- [.claude/agents/core-llm-router.md](.claude/agents/core-llm-router.md) — llm-router
- [.claude/agents/core-marketplace.md](.claude/agents/core-marketplace.md) — marketplace
- [.claude/agents/core-mcp.md](.claude/agents/core-mcp.md) — mcp
- [.claude/agents/core-settings.md](.claude/agents/core-settings.md) — settings
- [.claude/agents/core-system.md](.claude/agents/core-system.md) — system
- [.claude/agents/core-terminal.md](.claude/agents/core-terminal.md) — terminal
- [.claude/agents/review.md](.claude/agents/review.md) — 审查
- [.claude/agents/gc.md](.claude/agents/gc.md) — 清理

## Knowledge Base

- [Wiki Index](.claude/wiki/index.md) — 领域知识库
- [TDD Strategy](.claude/tdd/index.md) — 测试规格和骨架
- [Architecture](.claude/wiki/architecture/overview.md)
- [API Contracts](.claude/wiki/specs/api-contracts.md)
- [.claude/wiki/GRAPH_REPORT.md](.claude/wiki/GRAPH_REPORT.md) — god 节点 / 意外连接 / 建议问题
- [.claude/wiki/modules/claude-apps.md](.claude/wiki/modules/claude-apps.md)
- [.claude/wiki/modules/claude-bot.md](.claude/wiki/modules/claude-bot.md)
- [.claude/wiki/modules/coworks.md](.claude/wiki/modules/coworks.md)
- [.claude/wiki/modules/desktop.md](.claude/wiki/modules/desktop.md)
- [.claude/wiki/modules/devtools.md](.claude/wiki/modules/devtools.md)
- [.claude/wiki/modules/llm-router.md](.claude/wiki/modules/llm-router.md)
- [.claude/wiki/modules/marketplace.md](.claude/wiki/modules/marketplace.md)
- [.claude/wiki/modules/mcp.md](.claude/wiki/modules/mcp.md)
- [.claude/wiki/modules/settings.md](.claude/wiki/modules/settings.md)
- [.claude/wiki/modules/system.md](.claude/wiki/modules/system.md)
- [.claude/wiki/modules/terminal.md](.claude/wiki/modules/terminal.md)

## Milestones

- [Maintenance Foundation](.claude/milestones/m1.md) (6 tasks)

## Notes

- Tauri v2 IPC bridge between Vue 3 frontend and Rust backend
- Sidecar processes: Coworks SDK (Claude Agent SDK), Feishu WebSocket listener
- Plugin marketplace with Git-based distribution
- Test infrastructure in place: Vitest (frontend), Cargo test (Rust), 93.6% aggregate coverage on 8 targeted modules
- Knowledge graph hook installed; auto-rebuild on commit available via graphify hooks
- All M1 Foundation tasks completed
- Knowledge graph hook 已安装在 .claude/settings.json；它会在执行文件搜索前提醒 Claude 先查阅 GRAPH_REPORT.md。
- 首次建议运行 `python scripts/install_graphify_hooks.py --install` 开启 commit 自动重建
- 所有里程碑完成后，建议执行 `/wikiorganize` 整理 wiki 文档
