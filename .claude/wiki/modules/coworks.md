# coworks 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/coworks/`, `src/components/Coworks.vue`, `src/components/CoworksModelSettings.vue`, `src/stores/devtoolsStore.ts`, `src/composables/coworks/`, `src-tauri/src/coworks_config.rs`, `src-tauri/src/coworks_fs.rs`, `src-tauri/src/coworks_sidecar.rs`, `sidecar/coworks/`

## 职责

AI-powered collaboration hub with three modes: Chat (conversation), Code (development with file tree), and Cowork (full collaboration with Claude Agent SDK)

## 所属路径

- src/components/coworks/
- src/components/Coworks.vue
- src/components/CoworksModelSettings.vue
- src/stores/devtoolsStore.ts
- src/composables/coworks/
- src-tauri/src/coworks_config.rs
- src-tauri/src/coworks_fs.rs
- src-tauri/src/coworks_sidecar.rs
- sidecar/coworks/

## 实体

### CoworkSession (entity)

Active collaboration session with Claude

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### ChatMessage (entity)

Single message in a conversation

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### startSession `[入站]`

Start a new Cowork session

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { mode, projectPath, model? }
  ```
- **输出**:
  ```ts
  sessionId
  ```

### sendMessage `[入站]`

Send user message to Claude

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { sessionId, content }
  ```
- **输出**:
  ```ts
  stream of assistant chunks
  ```

### sidecar_spawn `[出站]`

Spawn Claude Agent SDK sidecar process

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { workDir, env }
  ```
- **输出**:
  ```ts
  process handle
  ```


## 场景

### User starts Chat mode

1. Select Chat mode
2. Enter message
3. Claude responds with streaming text

**边界情况**:
- API key missing
- Network timeout

### User starts Cowork mode

1. Select project directory
2. Sidecar spawns Claude Agent SDK
3. Agent executes tools
4. Results streamed back

**边界情况**:
- Sidecar crash
- Project not found


## 约束

### Performance

- First token latency < 2s for Chat mode


## 设计决策

- **Sidecar process for Claude Agent SDK**
  - 理由: Isolates Node.js runtime from Rust backend, prevents crash propagation
  - 备选方案: In-process WASM, HTTP microservice

## 相关任务

| ID | 标题 | 阶段 | 状态 | 里程碑 |
|---|---|---|---|---|
| T1-3 | Add unit tests for composables | exec | NOT_STARTED | M1 |

## 错误处理

- 方式: Result/Error propagation in Rust, try-catch in TypeScript
- 重试策略: no automatic retry for user-facing operations
- 降级行为: graceful degradation with user notification



## 相关依赖模块

- [claude-apps](claude-apps.md) — Lightweight app (dynamic function) system for creating, mana
- [claude-bot](claude-bot.md) — Autonomous Claude Bot engine with multi-channel integration 
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
