# MCP 管理功能说明

MCP（Model Context Protocol）管理器是 MindFuture Desktop 的核心功能之一，用于管理 Claude Code 的 MCP 服务器配置。

## 功能概述

MCP 管理器提供三个主要标签页：

### 1. 全局 MCP

显示和管理全局级别的 MCP 服务器配置。

**功能特性：**
- 查看所有全局 MCP 服务器列表
- 查看来自插件的 MCP 服务器（按市场分组显示）
- 启用/禁用全局 MCP 服务器（开关切换）
- 删除全局 MCP 服务器

**数据来源：**
- 全局 MCP：读取自 `~/.claude.json` 的 `mcpServers` 字段
- 插件 MCP：扫描 `~/.claude/plugins/marketplaces/` 目录下的 `.mcp.json` 文件

### 2. 项目 MCP

管理特定项目的 MCP 服务器配置。

**功能特性：**
- 左侧显示包含 MCP 配置的项目列表
- 右侧显示选中项目的 MCP 服务器详情
- 启用/禁用项目级 MCP 服务器
- 删除项目级 MCP 服务器

**数据来源：**
- 读取自 `~/.claude.json` 的 `projects.{path}.mcpServers` 字段

### 3. 安装 MCP

添加新的 MCP 服务器配置。

**功能特性：**
- 支持添加到**全局**或**指定项目**
- 配置服务器名称、命令、参数和环境变量
- 提供配置示例参考

**配置字段说明：**

| 字段 | 说明 | 示例 |
|------|------|------|
| 服务器名称 | MCP 服务器的唯一标识 | `my-mcp-server` |
| 命令 (command) | 启动 MCP 服务器的命令 | `npx` |
| 参数 (args) | 命令行参数，用逗号分隔 | `-y, @anthropic/mcp-server` |
| 环境变量 (env) | JSON 格式的环境变量 | `{"API_KEY": "your-key"}` |

## 配置文件结构

MCP 配置存储在 `~/.claude.json` 文件中：

```json
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@some/mcp-server"],
      "env": {
        "API_KEY": "your-api-key"
      }
    }
  },
  "projects": {
    "/path/to/project": {
      "mcpServers": {
        "project-server": {
          "command": "node",
          "args": ["server.js"]
        }
      }
    }
  },
  "enabledMcpjsonServers": [],
  "disabledMcpjsonServers": ["disabled-server"]
}
```

## 启用/禁用机制

- **启用状态**：服务器名称不在 `disabledMcpjsonServers` 列表中
- **禁用状态**：服务器名称在 `disabledMcpjsonServers` 列表中

切换开关时会自动更新对应的列表。

## 使用场景

### 添加全局 MCP 服务器

1. 打开 MCP 管理器
2. 切换到"安装 MCP"标签
3. 选择"全局"作为目标
4. 填写服务器配置信息
5. 点击"添加"按钮

### 添加项目级 MCP 服务器

1. 打开 MCP 管理器
2. 切换到"安装 MCP"标签
3. 选择"项目"作为目标
4. 从下拉菜单选择目标项目
5. 填写服务器配置信息
6. 点击"添加"按钮

### 管理现有服务器

1. 在"全局 MCP"或"项目 MCP"标签中找到目标服务器
2. 使用开关切换启用/禁用状态
3. 点击"删除"按钮移除服务器

## 注意事项

- 插件提供的 MCP 服务器由插件管理，不能在此直接删除
- 删除操作不可撤销，请谨慎操作
- 环境变量必须使用有效的 JSON 格式
- 项目路径列表来自 Claude Code 已访问过的项目记录
