# llm-router 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/LlmRouter.vue`, `src-tauri/src/llm_router.rs`

## 职责

Claude Code Router (CCR) for configuring LLM model routing, providers, and API endpoints

## 所属路径

- src/components/LlmRouter.vue
- src-tauri/src/llm_router.rs

## 实体

### RouterConfig (entity)

LLM routing configuration

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### getRouterConfig `[入站]`

Get current LLM routing config

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  RouterConfig
  ```

### setRouterConfig `[入站]`

Update LLM routing config

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  RouterConfig
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### User configures alternative LLM provider

1. Open LLM Router settings
2. Select provider and model
3. Enter API endpoint if custom
4. Save configuration

**边界情况**:
- Invalid API key
- Endpoint unreachable


## 约束

### Security

- API keys stored in OS credential store or encrypted local config, never exposed in UI
- Endpoint validation: only HTTPS URLs accepted for remote providers

### Data

- Router config persisted to local settings file, applied on next Claude session start


## 设计决策

- **Environment variable injection for routing**
  - 理由: Claude Code reads provider/model from env vars; router writes to the terminal env before session start
  - 备选方案: Direct config file modification, CLI flags

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
