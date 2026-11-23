# claude-bot 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/claudeBot/`, `src/stores/botStore.ts`, `src/composables/claudeBot/`, `src-tauri/src/claude_bot.rs`, `src-tauri/src/channels/`, `src-tauri/src/scheduler_heartbeat.rs`

## 职责

Autonomous Claude Bot engine with multi-channel integration (Discord, Telegram, Feishu) and task scheduling

## 所属路径

- src/components/claudeBot/
- src/stores/botStore.ts
- src/composables/claudeBot/
- src-tauri/src/claude_bot.rs
- src-tauri/src/channels/
- src-tauri/src/scheduler_heartbeat.rs

## 实体

### Bot (entity)

Autonomous Claude Bot with channel bindings and schedule

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### Channel (entity)

Message channel (Discord, Telegram, Feishu)

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### createBot `[入站]`

Create a new autonomous bot

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name, systemPrompt, channels, schedule? }
  ```
- **输出**:
  ```ts
  botId
  ```

### startBot `[入站]`

Start bot execution

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { botId }
  ```
- **输出**:
  ```ts
  void
  ```

### channel_send `[出站]`

Send message to external channel

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { channelType, target, message }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### Bot receives Feishu message and responds

1. Feishu WebSocket receives message
2. Filter evaluates message
3. Claude generates response
4. Response sent back to Feishu

**边界情况**:
- Filter blocks message
- Claude API timeout
- Channel disconnected


## 约束

### Performance

- Channel message round-trip (receive -> filter -> respond) < 5s
- Scheduler heartbeat interval: 60s

### Security

- Channel credentials stored in encrypted local storage, never logged
- Filter subprocess runs with minimal permissions, no filesystem write access

### Data

- Bot configurations persisted to local JSON, not synced across devices
- Message history retained per-channel up to configurable limit


## 设计决策

- **Filter via Claude Agent SDK one-shot script**
  - 理由: Replaced HTTP proxy with tokio subprocess for simplicity and reliability
  - 备选方案: HTTP proxy filter, In-process filter

## 相关任务

*暂无关联任务*

## 错误处理

- 方式: Result/Error propagation in Rust, try-catch in TypeScript
- 重试策略: no automatic retry for user-facing operations
- 降级行为: graceful degradation with user notification



## 相关依赖模块

- [claude-apps](claude-apps.md) — Lightweight app (dynamic function) system for creating, mana
- [coworks](coworks.md) — AI-powered collaboration hub with three modes: Chat (convers
- [desktop](desktop.md) — Virtual desktop environment with icon management, multi-wind
- [devtools](devtools.md) — Developer tools for inspecting Cowork sessions, agent hierar
- [llm-router](llm-router.md) — Claude Code Router (CCR) for configuring LLM model routing, 
- [marketplace](marketplace.md) — Plugin marketplace for discovering, installing and managing 
- [mcp](mcp.md) — Model Context Protocol (MCP) server configuration and manage
- [settings](settings.md) — Application settings management including theme, environment
- [system](system.md) — System dependency management (Node.js, Git, UV), auto-update
- [terminal](terminal.md) — Cross-platform pseudo-terminal (PTY) with xterm.js UI, suppo

## 相关社区

_(无图社区)_
