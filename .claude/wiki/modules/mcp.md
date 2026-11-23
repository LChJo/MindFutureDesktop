# mcp 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/McpManager.vue`, `src-tauri/src/mcp.rs`

## 职责

Model Context Protocol (MCP) server configuration and management, supporting global and project-level servers

## 所属路径

- src/components/McpManager.vue
- src-tauri/src/mcp.rs

## 实体

### McpServer (entity)

MCP server configuration entry

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### listMcpServers `[入站]`

List configured MCP servers

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { scope? }
  ```
- **输出**:
  ```ts
  McpServer[]
  ```

### addMcpServer `[入站]`

Add MCP server config

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name, command, args, scope }
  ```
- **输出**:
  ```ts
  void
  ```

### removeMcpServer `[入站]`

Remove MCP server config

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { name, scope }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### User adds a project-level MCP server

1. Open MCP Manager
2. Click Add Server
3. Enter server name and command
4. Server config written to project .claude/settings.json

**边界情况**:
- Duplicate name
- Invalid command path


## 约束

### Security

- MCP server commands validated against allowlist patterns
- Project-scoped servers isolated from global config

### Data

- Global servers stored in ~/.claude/settings.json
- Project servers stored in .claude/settings.json within project root


## 设计决策

- **Dual-scope configuration (global + project)**
  - 理由: Matches Claude Code's native settings.json structure, project-level overrides for team sharing
  - 备选方案: Single global config, Database-backed registry

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
