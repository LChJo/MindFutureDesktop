# terminal 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/Terminal.vue`, `src/components/TerminalFilePanel.vue`, `src/components/TerminalSkillPanel.vue`, `src/stores/terminalStore.ts`, `src/composables/useTerminalPty.ts`, `src-tauri/src/pty.rs`, `src-tauri/src/node_pty.rs`

## 职责

Cross-platform pseudo-terminal (PTY) with xterm.js UI, supporting both Unix portable-pty and Windows ConPTY

## 所属路径

- src/components/Terminal.vue
- src/components/TerminalFilePanel.vue
- src/components/TerminalSkillPanel.vue
- src/stores/terminalStore.ts
- src/composables/useTerminalPty.ts
- src-tauri/src/pty.rs
- src-tauri/src/node_pty.rs

## 实体

### PtySession (entity)

Active PTY session with input/output streams

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### pty_spawn `[入站]`

Spawn a new PTY session

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { cwd, cols, rows, env? }
  ```
- **输出**:
  ```ts
  sessionId
  ```
- **错误码**: `SPAWN_FAILED`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_

### pty_write `[入站]`

Write data to PTY stdin

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { sessionId, data }
  ```
- **输出**:
  ```ts
  void
  ```
- **错误码**: `SESSION_NOT_FOUND`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_

### pty_resize `[入站]`

Resize PTY dimensions

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { sessionId, cols, rows }
  ```
- **输出**:
  ```ts
  void
  ```

### pty_kill `[入站]`

Kill PTY session

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { sessionId }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### User opens terminal tab

1. Click terminal icon or shortcut
2. PTY spawned with default shell
3. xterm.js connects to PTY output stream
4. User types commands

**边界情况**:
- Shell not found
- PTY spawn fails on Windows


## 约束

### Performance

- PTY I/O latency < 10ms


## 设计决策

- **Dual PTY backend: portable-pty (Unix) + ConPTY (Windows)**
  - 理由: Native performance on each platform
  - 备选方案: node-pty via sidecar

## 相关任务

| ID | 标题 | 阶段 | 状态 | 里程碑 |
|---|---|---|---|---|
| T1-4 | Add Rust backend unit tests | exec | NOT_STARTED | M1 |

## 错误处理

- 方式: Result/Error propagation in Rust, try-catch in TypeScript
- 重试策略: no automatic retry for user-facing operations
- 降级行为: graceful degradation with user notification



## 相关依赖模块

- [claude-bot](claude-bot.md) — Autonomous Claude Bot engine with multi-channel integration 
- [coworks](coworks.md) — AI-powered collaboration hub with three modes: Chat (convers

## 相关社区

_(无图社区)_
