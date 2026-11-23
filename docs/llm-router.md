# LLM Router (LLM 路由)

LLM Router 是 Moonton Claude Desktop 内置的模型路由服务，基于 CCR (Claude Code Router) 实现。它允许用户将 Claude Code 的请求透明地路由到不同的 LLM 提供商（Claude、OpenAI、Gemini 等），无需修改 Claude Code 本身的配置。

## 核心概念

| 概念 | 说明 |
|------|------|
| CCR | Claude Code Router，内嵌的 Node.js 路由代理服务 |
| Provider | 模型提供商配置，包含 API 地址和密钥 |
| Router | 路由规则，定义不同场景使用哪个 Provider 的哪个模型 |
| Transformer | 请求/响应转换器，用于注入推理参数等 |
| mcd CLI | 封装了路由环境变量的 Claude Code 启动命令 |

## 功能入口

桌面上的 **LLM 路由** 图标（`fa-solid fa-route`）打开路由管理界面，界面分为两个 Tab：

- **配置 Tab**：模型提供商选择、mcd CLI 安装、推理深度设置、调试面板
- **路由器 Tab**：内嵌 CCR Web UI（`http://127.0.0.1:3456/ui`），提供可视化的路由规则编辑

## 工作原理

```
┌─────────────────────────────────────────────────────┐
│               Claude Code (终端)                     │
│   ANTHROPIC_BASE_URL=http://127.0.0.1:3456/v1      │
│   ANTHROPIC_MODEL=gpt-5.2                           │
└──────────────────────┬──────────────────────────────┘
                       │ API 请求
                       ▼
┌─────────────────────────────────────────────────────┐
│            CCR (Claude Code Router)                  │
│            http://127.0.0.1:3456                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  Router 规则                                   │  │
│  │  ├── default:  example,gpt-5.2               │  │
│  │  ├── think:    example,gpt-5.2               │  │
│  │  ├── image:    example,gemini-3-pro-image    │  │
│  │  └── ...                                      │  │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Transformer                                   │  │
│  │  ├── stripreasoning (移除推理标签)             │  │
│  │  └── reasoning { effort: "medium" }           │  │
│  └───────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────┘
                       │ 转发请求
                       ▼
┌─────────────────────────────────────────────────────┐
│         实际 LLM API (OpenAI / Gemini)     │
│         https://api.example.com/v1/...           │
└─────────────────────────────────────────────────────┘
```

1. Claude Code 发送 API 请求到本地 CCR 代理 (`127.0.0.1:3456`)
2. CCR 根据路由规则匹配 Provider 和模型
3. Transformer 对请求进行预处理（如注入推理深度参数）
4. CCR 将处理后的请求转发到实际的 LLM API
5. 响应经过逆向 Transformer 处理后返回给 Claude Code

首次启动路由服务后，在 CCR 控制台添加供应商，并设置 `Router.default` 后再转发请求。

## 配置文件

配置位于 `~/.MoontonClaudeDesktop/ccr-config/config.json`：

```json
{
  "PORT": 3456,
  "HOST": "127.0.0.1",
  "LOG": false,
  "LOG_LEVEL": "debug",
  "API_TIMEOUT_MS": "600000",
  "Router": {
    "default": "example,gpt-5.2",
    "background": "example,gpt-5.2",
    "think": "example,gpt-5.2",
    "longContext": "example,gpt-5.2",
    "longContextThreshold": 60000,
    "webSearch": "example,gpt-5.2",
    "image": "example,gemini-3-pro-image-preview"
  },
  "Providers": [
    {
      "name": "example",
      "api_base_url": "https://api.example.com/v1/chat/completions",
      "api_key": "<your-api-key>",
      "models": [
        "Opus",
        "gpt-5.1-codex",
        "gpt-5.2",
        "gpt-5.2-codex",
        "gpt-5.3",
        "gpt-5.3-codex",
        "gemini-3-pro-preview",
        "gemini-3-pro-image-preview"
      ],
      "transformer": {
        "use": [
          "stripreasoning",
          ["reasoning", { "effort": "medium" }]
        ]
      }
    }
  ]
}
```

### 配置字段说明

#### 顶层配置

| 字段 | 类型 | 说明 |
|------|------|------|
| `PORT` | number | CCR 监听端口，固定 `3456` |
| `HOST` | string | 绑定地址，固定 `127.0.0.1` |
| `LOG` | boolean | 是否启用日志 |
| `LOG_LEVEL` | string | 日志级别 |
| `API_TIMEOUT_MS` | string | API 超时时间（毫秒），默认 10 分钟 |

#### Router 规则

| 字段 | 说明 |
|------|------|
| `default` | 默认路由，格式 `provider,model` |
| `background` | 后台任务路由 |
| `think` | 需要深度思考的请求路由 |
| `longContext` | 长上下文请求路由 |
| `longContextThreshold` | 长上下文阈值（token 数） |
| `webSearch` | 网络搜索请求路由 |
| `image` | 图像相关请求路由 |

#### Provider 配置

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 提供商名称标识 |
| `api_base_url` | string | API 端点地址 |
| `api_key` | string | API 密钥，支持从 `ANTHROPIC_AUTH_TOKEN` 环境变量自动填充 |
| `models` | string[] | 该提供商支持的模型列表 |
| `transformer` | object | 请求/响应转换器配置 |

#### Transformer

Transformer 对经过 CCR 的请求进行预处理。当前支持的转换器：

| 转换器 | 说明 |
|--------|------|
| `stripreasoning` | 从响应中移除推理标签 |
| `reasoning` | 注入推理深度参数，`effort` 可选值：`none`、`low`、`medium`、`high` |

## 模型提供商

界面提供三个预设提供商，用户可选择并自定义模型名称：

| 提供商 | 默认模型 | 图标 |
|--------|----------|------|
| Claude | Sonnet | `fa-solid fa-robot` |
| OpenAI | gpt-5.2 | `fa-solid fa-brain` |
| Gemini | gemini-3-pro-preview | `fa-solid fa-gem` |

选择提供商后，`mcd` 命令启动的 Claude Code 将自动使用对应模型。模型名称支持内联编辑。

## 推理深度 (Thinking Level)

当选择 OpenAI 提供商时，可配置推理深度，控制 GPT-5.2 的思考过程：

| 级别 | 标识 | 说明 |
|------|------|------|
| 无 | `none` | 不使用推理 |
| 低 | `low` | 快速响应，适合简单任务 |
| 中等 | `medium` | 平衡速度与深度（默认） |
| 高 | `high` | 深度推理，适合复杂任务 |

修改后需重启服务生效，会写入 Transformer 的 `reasoning.effort` 字段。

## mcd CLI

`mcd` 是封装了 LLM Router 环境变量的 Claude Code 启动命令。

### 安装方式

1. 在 LLM Router 配置页点击 **安装 mcd CLI**
2. 安装 Claude Code 时自动附带安装 mcd CLI

安装路径：
- Windows: `%APPDATA%/npm/`
- macOS: `~/.npm-global/`

### 工作机制

`mcd` 命令启动时自动注入环境变量：
- `ANTHROPIC_BASE_URL` -> `http://127.0.0.1:3456/v1`
- `ANTHROPIC_MODEL` -> 用户选择的模型名称

这样 Claude Code 的所有请求都会经过本地 CCR 代理转发。

### 终端 PATH 注入

应用启动终端时会自动将 CCR 和 mcd 的可执行文件目录注入 `PATH`，用户无需手动配置即可在内置终端中直接使用 `ccr` 和 `mcd` 命令。

## 服务生命周期

### 启动流程

1. 确保配置文件存在（不存在则生成默认配置）
2. 自动修复配置：填充缺失的 Provider `name`、`api_key`
3. 注入 `ANTHROPIC_AUTH_TOKEN` 作为 API Key（如有）
4. 更新 Transformer 推理深度参数
5. 通过 Node.js 启动 `ccr/cli.cjs start` 作为后台进程
6. 状态轮询确认服务就绪（端口 3456 可连接）

### 状态检测

通过 TCP 连接 `127.0.0.1:3456` 检测服务是否运行。UI 提供两种轮询频率：
- 配置 Tab：每 10 秒慢速轮询
- 路由器 Tab（iframe 可见时）：每 2 秒快速轮询

### 自动重启 (Watchdog)

当检测到服务意外停止（如通过 CCR Web UI 的"保存并重启"）时，自动触发重启：

1. 检测到 `wasRunning && !isRunning` 且非手动操作
2. 调用 `restart_llm_router` 并设置 `skip_config_update=true`（保留用户在 Web UI 的配置修改）
3. 最多自动重启 3 次，超过则提示用户手动检查
4. 重启成功后自动刷新 iframe

### 手动重启

头部的"重启服务"按钮执行完整重启流程：
1. 停止当前 CCR 进程
2. 等待 500ms 释放端口
3. 根据设置更新 `config.json`（auth_token + thinking_level）
4. 重新启动 CCR 进程

### 停止

通过 `ccr/cli.cjs stop` 命令停止服务。

## CCR 资源路径

| 环境 | 路径 |
|------|------|
| 生产 (Windows) | `{exe_dir}/resources/ccr/cli.cjs` |
| 生产 (macOS) | `{exe_dir}/../Resources/resources/ccr/cli.cjs` |
| 开发 | `src-tauri/resources/ccr/cli.cjs` |

Windows 下使用 `CREATE_NO_WINDOW` 标志隐藏控制台窗口，并处理 `\\?\` UNC 路径前缀以兼容 Node.js。

## 调试

点击 **调试** 按钮展开调试面板，显示：

| 信息 | 说明 |
|------|------|
| Node.js 路径 | 当前使用的 Node.js 可执行文件路径 |
| CCR 路径 | `cli.cjs` 文件路径 |
| CCR Bin 路径 | `ccr` / `ccr.cmd` 所在目录 |
| 配置文件路径 | `config.json` 完整路径 |
| ANTHROPIC_MODEL | Rust 进程和 Settings Store 中的值 |
| ANTHROPIC_BASE_URL | Rust 进程环境变量中的值 |
| 配置文件内容 | 完整的 `config.json` 内容 |

## 设置持久化

LLM Router 的用户偏好独立于环境变量存储在 `settingsStore.llmRouterConfig`：

```typescript
interface LlmRouterConfig {
  selectedProviderId: string           // 'claude' | 'openai' | 'gemini'
  modelProviders: Record<string, string>  // { claude: 'Sonnet', openai: 'gpt-5.2', ... }
  thinkingLevel: 'none' | 'low' | 'medium' | 'high'
}
```

持久化到 `~/.MoontonClaudeDesktop/config/settings.json`。

## 架构总览

```
┌─────────────────────────────────────────────────────────┐
│                   LlmRouter.vue                          │
│  ┌────────────────────┐  ┌────────────────────────────┐ │
│  │  配置 Tab           │  │  路由器 Tab                │ │
│  │  ├ 模型提供商选择   │  │  └ iframe -> CCR Web UI   │ │
│  │  ├ mcd CLI 安装     │  │    http://127.0.0.1:3456  │ │
│  │  ├ 推理深度设置     │  │    /ui                    │ │
│  │  └ 调试面板         │  │                            │ │
│  └────────┬───────────┘  └──────────┬─────────────────┘ │
│           │                          │                   │
│           ▼                          │                   │
│  ┌────────────────────┐             │                   │
│  │  settingsStore     │             │                   │
│  │  .llmRouterConfig  │             │                   │
│  └────────┬───────────┘             │                   │
└───────────┼─────────────────────────┼───────────────────┘
            │                         │
            ▼                         ▼
┌───────────────────────────────────────────────────────┐
│            Rust: llm_router.rs                         │
│  ├── check_llm_router_status()   TCP 端口检测          │
│  ├── start_llm_router()          启动 CCR + 配置修复   │
│  ├── stop_llm_router()           停止 CCR              │
│  ├── restart_llm_router()        停止 -> 配置 -> 启动  │
│  ├── get_ccr_debug_info()        调试信息收集           │
│  ├── get_ccr_bin_path()          CCR 可执行文件路径     │
│  └── get_mcd_bin_path()          mcd CLI 路径           │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│        CCR Process (Node.js)                           │
│        ccr/cli.cjs start                               │
│        监听 127.0.0.1:3456                              │
│        ├── /v1/*          API 代理                     │
│        └── /ui            Web 管理界面                  │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│  Rust: pty.rs                                          │
│  终端 PATH 注入                                         │
│  ├── CCR bin 路径  ->  用户可使用 `ccr` 命令            │
│  └── mcd bin 路径  ->  用户可使用 `mcd` 命令            │
└───────────────────────────────────────────────────────┘
```

## 关键源文件

| 文件 | 职责 |
|------|------|
| `src/components/LlmRouter.vue` | 主界面：配置 Tab + 路由器 Tab + 状态管理 + Watchdog |
| `src/stores/settingsStore.ts` | `LlmRouterConfig` 定义和持久化 |
| `src-tauri/src/llm_router.rs` | Rust 后端：服务启停、配置管理、调试信息、路径解析 |
| `src-tauri/src/nodejs.rs` | mcd CLI 安装逻辑、Node.js 路径解析 |
| `src-tauri/src/pty.rs` | 终端 PATH 注入（CCR + mcd 可执行文件路径） |
| `src-tauri/resources/ccr/cli.cjs` | CCR 主程序（内嵌资源） |
| `~/.MoontonClaudeDesktop/ccr-config/config.json` | 运行时配置文件 |
