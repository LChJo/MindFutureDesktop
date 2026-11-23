# devtools 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/devtools/`, `src/components/DevTools.vue`, `src/stores/devtoolsStore.ts`, `src-tauri/src/devtools.rs`

## 职责

Developer tools for inspecting Cowork sessions, agent hierarchies, tool invocations, and performance metrics

## 所属路径

- src/components/devtools/
- src/components/DevTools.vue
- src/stores/devtoolsStore.ts
- src-tauri/src/devtools.rs

## 实体

### ToolCall (entity)

Recorded tool invocation from a Cowork session

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### getSessionMetrics `[入站]`

Get metrics for a Cowork session

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { sessionId }
  ```
- **输出**:
  ```ts
  { toolCalls, tokenUsage, duration }
  ```


## 场景

### Developer inspects Cowork session

1. Open DevTools panel
2. Select session from sidebar
3. View agent tree, tool calls, metrics

**边界情况**:
- Session still in progress
- Large session data


## 约束

### Performance

- Session tree rendering must handle 100+ tool calls without UI jank
- Metrics aggregation computed lazily on panel open

### Data

- Session data is read-only from Cowork session stores, DevTools does not persist its own state


## 设计决策

- **Read-only observer pattern**
  - 理由: DevTools reads from devtoolsStore shared with Coworks, avoiding data duplication
  - 备选方案: Separate data collection pipeline, WebSocket stream

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
