# settings 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/Settings.vue`, `src/components/ClaudeAliases.vue`, `src/stores/settingsStore.ts`, `src-tauri/src/window_state.rs`, `src-tauri/src/app_paths.rs`

## 职责

Application settings management including theme, environment variables, Claude aliases, window state persistence, and data directory paths

## 所属路径

- src/components/Settings.vue
- src/components/ClaudeAliases.vue
- src/stores/settingsStore.ts
- src-tauri/src/window_state.rs
- src-tauri/src/app_paths.rs

## 实体

### AppSettings (entity)

Global application settings

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### ClaudeAlias (entity)

Named alias for launching Claude with specific configuration

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### getSettings `[入站]`

Load application settings

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  AppSettings
  ```

### saveSettings `[入站]`

Persist settings changes

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  AppSettings
  ```
- **输出**:
  ```ts
  void
  ```

### getWindowState `[入站]`

Restore window position and size

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  WindowState
  ```


## 场景

### User creates a Claude alias

1. Open Settings > Claude Aliases
2. Click Add Alias
3. Enter alias name and env overrides
4. Alias available in terminal after restart

**边界情况**:
- Empty alias name
- Conflicting alias name
- Env variable inheritance from system


## 约束

### Performance

- Settings load/save < 50ms (local file I/O)
- Window state restore on app launch < 100ms

### Security

- Environment variables may contain API keys; never logged or displayed in plain text
- Alias env overrides explicitly injected (empty string for unset) to prevent system env leakage

### Data

- Settings persisted to JSON in user data directory
- Window state (position, size, maximized) saved on close, restored on launch


## 设计决策

- **Explicit empty env injection for unconfigured aliases**
  - 理由: Prevents inheriting system ENV values when alias has no overrides
  - 备选方案: Skip injection, Inherit all system values

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
