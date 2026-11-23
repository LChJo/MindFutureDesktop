# claude-apps 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/claudeApps/`, `src/components/DynamicFunctionCreator.vue`, `src/components/DynamicFunctionHost.vue`, `src/stores/dynamicFunctionStore.ts`, `src/composables/claudeApps/`, `src-tauri/src/dynamic_function.rs`

## 职责

Lightweight app (dynamic function) system for creating, managing and running user-defined mini-apps with optional backend support and conversation history

## 所属路径

- src/components/claudeApps/
- src/components/DynamicFunctionCreator.vue
- src/components/DynamicFunctionHost.vue
- src/stores/dynamicFunctionStore.ts
- src/composables/claudeApps/
- src-tauri/src/dynamic_function.rs

## 实体

### DynamicFunction (entity)

User-created lightweight app with HTML/JS frontend and optional Node.js backend

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### BackendStatus (value_object)

Runtime status of a dynamic function backend process

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### ConversationIndexItem (entity)

Index entry for a Claude Apps conversation history

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### df_list_functions `[入站]`

List all installed dynamic functions

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  DynamicFunctionInfo[]
  ```

### df_create_function `[入站]`

Create a new dynamic function from template

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name, displayName, description, icon }
  ```
- **输出**:
  ```ts
  DynamicFunctionInfo
  ```

### df_start_backend `[入站]`

Start the backend process for a dynamic function

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name }
  ```
- **输出**:
  ```ts
  { port }
  ```

### df_stop_backend `[入站]`

Stop the backend process

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name }
  ```
- **输出**:
  ```ts
  void
  ```

### mcd.exec `[入站]`

Execute shell command from within a dynamic function (sandboxed)

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { command, args, cwd? }
  ```
- **输出**:
  ```ts
  { stdout, stderr, exitCode }
  ```

### mcd.kill `[入站]`

Kill a running process started by mcd.exec

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { pid }
  ```
- **输出**:
  ```ts
  void
  ```

### mcd.openExternal `[入站]`

Open URL or file with system default handler

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { url }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### User creates a new lightweight app

1. Open DynamicFunctionCreator
2. Fill in name, description, icon
3. App scaffold created on filesystem
4. Desktop icon added automatically

**边界情况**:
- Name collision
- Filesystem permission denied

### User runs a lightweight app

1. Double-click app icon or select from list
2. DynamicFunctionHost loads app HTML
3. If backend enabled, start backend process
4. App communicates via postMessage API

**边界情况**:
- Backend startup failure
- Port conflict
- postMessage origin mismatch

### User views conversation history

1. Open ClaudeAppsView
2. Select app from sidebar
3. Browse past conversations
4. Continue or delete conversation

**边界情况**:
- Corrupted conversation file
- App deleted but conversations remain


## 约束

### Security

- postMessage origin validation required
- mcd.exec limited to app working directory


## 设计决策

- **postMessage bridge for app-host communication**
  - 理由: Secure iframe isolation with controlled API surface via mcd.exec/mcd.kill/mcd.openExternal
  - 备选方案: Direct DOM access, WebSocket bridge

## 相关任务

*暂无关联任务*

## 错误处理

- 方式: Result/Error propagation in Rust, try-catch in TypeScript
- 重试策略: no automatic retry for user-facing operations
- 降级行为: graceful degradation with user notification



## 相关依赖模块

- [claude-bot](claude-bot.md) — Autonomous Claude Bot engine with multi-channel integration 
- [coworks](coworks.md) — AI-powered collaboration hub with three modes: Chat (convers

## 相关社区

_(无图社区)_
