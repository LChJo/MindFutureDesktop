# marketplace 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/market/`, `src/components/PluginMarket.vue`, `src/components/PluginMarketFull.vue`, `src/stores/marketplaceStore.ts`, `src/stores/pluginStore.ts`, `src-tauri/src/marketplace.rs`, `src-tauri/src/skills.rs`

## 职责

Plugin marketplace for discovering, installing and managing Claude Code extensions (skills, agents, commands, MCP servers)

## 所属路径

- src/components/market/
- src/components/PluginMarket.vue
- src/components/PluginMarketFull.vue
- src/stores/marketplaceStore.ts
- src/stores/pluginStore.ts
- src-tauri/src/marketplace.rs
- src-tauri/src/skills.rs

## 实体

### Plugin (entity)

Installable Claude Code extension package

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### searchPlugins `[入站]`

Search marketplace catalog

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { query, type?, page? }
  ```
- **输出**:
  ```ts
  Plugin[]
  ```

### installPlugin `[入站]`

Install plugin from source

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { pluginId }
  ```
- **输出**:
  ```ts
  void
  ```
- **错误码**: `INSTALL_FAILED`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_

### uninstallPlugin `[入站]`

Remove installed plugin

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { pluginId }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### User installs a plugin

1. Browse or search marketplace
2. Click install on plugin card
3. Plugin cloned from Git source
4. Plugin registered in local config

**边界情况**:
- Git clone fails
- Invalid plugin manifest
- Disk space insufficient


## 约束

### Performance

- Marketplace catalog fetch with local cache, refresh interval: 1 hour
- Plugin install (git clone) timeout: 60s

### Security

- Plugins run in user context with Claude Code's permission model
- Plugin source URLs validated before clone

### Data

- Installed plugins tracked in local registry JSON
- Plugin assets stored under user data directory


## 设计决策

- **Git-based plugin distribution**
  - 理由: Leverages existing Git infrastructure, no custom registry needed
  - 备选方案: npm registry, Custom HTTP API

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
