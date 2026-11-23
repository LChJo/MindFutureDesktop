# Claude Apps

Claude Apps 是 Moonton Claude Desktop 内置的轻量级应用开发平台，允许用户通过自然语言描述，由 Claude 自动生成可运行的 Web 小应用，并直接在桌面环境中以窗口形式打开和使用。

## 核心概念

| 概念 | 说明 |
|------|------|
| App | 一个独立的小型 Web 应用，由前端 HTML/CSS/JS 组成，可选附带 Node.js 后端 |
| Manifest | 应用元数据文件 (`manifest.json`)，定义名称、图标、窗口大小、能力声明等 |
| Conversation | 每个 App 对应一个持久化的对话，用于创建和迭代维护该应用 |
| Bridge API | iframe 内的 `window.mcd` 对象，提供与宿主环境的安全通信接口 |

## 功能入口

桌面上的 **Claude Apps** 图标（`fa-solid fa-cubes`）打开 Claude Apps 管理界面，界面分为左右两栏：

- **左栏**：应用列表，展示已创建的所有应用及其版本、后端状态；提供运行、打开目录、删除等操作
- **右栏**：对话面板，与 Claude 交互来创建或修改选中的应用

## 应用生命周期

### 1. 创建应用

点击左上角的 **+** 按钮，在弹窗中用自然语言描述需求（如"一个简单的天气查询应用"），Claude 会自动：

1. 在 `~/.MoontonClaudeDesktop/dynamic-functions/` 下创建应用目录
2. 生成 `manifest.json`、`frontend/index.html`、`frontend/app.js`、`frontend/style.css`
3. 如需后端，额外生成 `backend/server.js`
4. 自动将应用图标添加到桌面

### 2. 迭代修改

选中已有应用后，通过对话面板继续描述修改需求，Claude 会读取并修改应用文件。每个应用对应唯一的对话上下文，Claude 了解当前应用的完整信息。

### 3. 运行应用

通过列表中的播放按钮或桌面图标打开应用。应用在沙箱化的 iframe 中运行，通过 `srcdoc` 注入前端代码。

### 4. 删除应用

列表中的删除按钮会：关闭运行中的窗口 -> 终止后端进程 -> 删除文件目录 -> 清理对话历史 -> 移除桌面图标。

## 目录结构

```
~/.MoontonClaudeDesktop/dynamic-functions/
└── {app-name}/
    ├── manifest.json           # 应用元数据
    ├── frontend/
    │   ├── index.html          # 前端入口
    │   ├── app.js              # 前端逻辑
    │   └── style.css           # 样式
    └── backend/                # 可选
        └── server.js           # Node.js HTTP 服务
```

对话历史持久化在 `~/.MoontonClaudeDesktop/coworks/claude-apps-chats/`。

## manifest.json 规范

```json
{
  "name": "my-app",
  "displayName": "我的应用",
  "version": "1.0.0",
  "description": "应用描述",
  "icon": "fa-solid fa-cube",
  "window": {
    "width": 800,
    "height": 600,
    "minWidth": 400,
    "minHeight": 300
  },
  "capabilities": {
    "network": false,
    "localStorage": false
  },
  "backend": {
    "enabled": false,
    "dependencies": {}
  }
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 应用目录名，仅允许字母数字、连字符、下划线，最长 100 字符 |
| `displayName` | string | 是 | 显示名称，支持别名 `display_name`、`title` |
| `version` | string | 否 | 版本号，默认 `1.0.0` |
| `description` | string | 否 | 应用描述 |
| `icon` | string | 否 | Font Awesome 图标类名（如 `fa-solid fa-cube`），非 FA 类名回退为默认图标 |
| `window` | object | 否 | 窗口尺寸配置，默认 800x600 / 最小 400x300 |
| `capabilities` | object/array | 否 | 能力声明，支持对象格式或数组格式 |
| `backend` | object | 否 | 后端配置，`enabled: true` 启用 Node.js 后端 |

### capabilities 格式

对象格式：
```json
{ "network": true, "localStorage": true }
```

数组格式：
```json
["network", "storage"]
```

支持的能力标识：`network`/`fetch`/`http`（网络）、`localStorage`/`storage`/`local_storage`（本地存储）。

## Bridge API (window.mcd)

应用在 iframe 中通过 `window.mcd` 对象与宿主通信，该对象在页面加载前由宿主注入。

### mcd.backend(path, options)

请求后端 API（仅后端启用时可用）。

```javascript
const data = await mcd.backend('/api/weather', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ city: '北京' })
})
```

内部转发到 `http://127.0.0.1:{port}{path}`，使用 `fetch` 发起请求。

### mcd.invoke(command, args)

调用宿主提供的命令，返回 Promise。

可用命令：

| 命令 | 参数 | 说明 |
|------|------|------|
| `mcd:readFile` | `{ path: string }` | 读取应用目录内的文件（相对路径） |
| `mcd:writeFile` | `{ path: string, content: string }` | 写入应用目录内的文件（相对路径） |

```javascript
// 读取配置文件
const config = await mcd.invoke('mcd:readFile', { path: 'data/config.json' })

// 写入数据
await mcd.invoke('mcd:writeFile', {
  path: 'data/result.json',
  content: JSON.stringify(result)
})
```

路径安全限制：禁止 `..` 路径穿越、禁止绝对路径、禁止空字节。

### mcd.setTitle(title)

设置窗口标题。

```javascript
mcd.setTitle('天气查询 - 北京')
```

### mcd.notify(message, level)

显示通知消息。

```javascript
mcd.notify('数据已保存', 'info')
mcd.notify('保存失败', 'error')
```

### mcd.on(event, callback) / mcd.off(event, callback)

监听/取消监听宿主事件。

```javascript
mcd.on('refresh', () => loadData())
```

## 后端服务

当 `manifest.json` 中 `backend.enabled` 为 `true` 时，应用支持 Node.js 后端。

### 运行机制

1. 应用窗口打开时自动启动后端（`backend/server.js`）
2. 系统自动分配可用端口，通过 `PORT` 和 `HOST` 环境变量传入
3. 后端采用引用计数管理：多个窗口共享同一后端实例
4. 窗口关闭时引用计数减 1，归零则终止进程
5. 应用删除时强制终止后端进程

### server.js 模板

```javascript
const http = require('http')

const PORT = process.env.PORT || 3000
const HOST = process.env.HOST || '127.0.0.1'

const server = http.createServer((req, res) => {
  res.setHeader('Content-Type', 'application/json')
  res.setHeader('Access-Control-Allow-Origin', '*')

  if (req.url === '/api/hello') {
    res.end(JSON.stringify({ message: 'Hello from backend' }))
  } else {
    res.statusCode = 404
    res.end(JSON.stringify({ error: 'Not found' }))
  }
})

server.listen(PORT, HOST, () => {
  console.log(`Server running at http://${HOST}:${PORT}`)
})
```

### 后端状态管理

后端有 5 种状态：`stopped` -> `starting` -> `running` -> `stopping` -> `stopped`，以及 `error` 状态。在应用列表中通过服务器图标展示当前状态，支持手动启停。

## 安全模型

### iframe 沙箱

应用运行在 `sandbox` 属性受限的 iframe 中：

- 默认：`allow-scripts`（允许执行 JS）
- 声明 `localStorage` 能力时：追加 `allow-same-origin`
- 网络访问通过 `mcd.backend()` 桥接，不直接暴露 `fetch`

### 文件系统隔离

- 每个应用只能读写自己目录下的文件
- 路径穿越检查（`..`）、绝对路径拒绝
- Rust 端使用 `canonicalize()` 验证解析后路径在应用目录内
- 文件大小限制：单文件 10MB，总计 50MB，manifest 100KB

### 进程隔离

后端进程以 `Stdio::null()` 作为 stdin，stdout/stderr 由日志线程消费。Windows 下使用 `CREATE_NO_WINDOW` 标志隐藏控制台窗口。

## 架构总览

```
┌─────────────────────────────────────────────────────────┐
│                   ClaudeAppsView.vue                    │
│  ┌──────────────┐  ┌─────────────────────────────────┐  │
│  │ ClaudeApps   │  │       ChatPanel.vue             │  │
│  │ List.vue     │  │  (对话输入 / 消息展示 / 模型选择) │  │
│  └──────┬───────┘  └──────────────┬──────────────────┘  │
│         │                         │                     │
│         ▼                         ▼                     │
│  ┌──────────────────────────────────────────────────┐   │
│  │            useClaudeApps.ts (Facade)             │   │
│  │  ┌─────────────────┐  ┌──────────────────────┐   │   │
│  │  │ useSession.ts   │  │ useClaudeAppsHistory │   │   │
│  │  │ (Sidecar 会话)  │  │ (对话持久化)         │   │   │
│  │  └────────┬────────┘  └──────────┬───────────┘   │   │
│  └───────────┼──────────────────────┼───────────────┘   │
└──────────────┼──────────────────────┼───────────────────┘
               │                      │
               ▼                      ▼
┌──────────────────────┐  ┌─────────────────────────────┐
│   Claude SDK         │  │  Rust: coworks_config.rs    │
│   (Sidecar 进程)     │  │  claude_apps_*_conversation │
└──────────────────────┘  └─────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              DynamicFunctionHost.vue                     │
│  ┌──────────────────────────────────────────────────┐   │
│  │  sandboxed iframe (srcdoc)                       │   │
│  │  ┌────────────────────────────────────────────┐  │   │
│  │  │ window.mcd  (Bridge API)                   │  │   │
│  │  │  .backend()  .invoke()  .setTitle()        │  │   │
│  │  └────────────────────┬───────────────────────┘  │   │
│  └───────────────────────┼──────────────────────────┘   │
│                          │ postMessage                   │
│                          ▼                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Message Handler (readFile / writeFile)           │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│           Rust: dynamic_function.rs                      │
│  DynamicFunctionManager                                  │
│  ├── list_functions()    扫描目录，解析 manifest         │
│  ├── save_function()     保存应用文件                    │
│  ├── start_backend()     启动 Node.js 后端（引用计数）   │
│  ├── stop_backend()      停止后端（引用计数递减）        │
│  └── delete_function()   强制停止 + 删除目录             │
└─────────────────────────────────────────────────────────┘
```

## 关键源文件

| 文件 | 职责 |
|------|------|
| `src/composables/claudeApps/useClaudeApps.ts` | Facade composable，编排应用列表、对话、Sidecar 会话 |
| `src/composables/claudeApps/useClaudeAppsHistory.ts` | 对话历史持久化，通过 Rust 命令读写 JSON 文件 |
| `src/components/claudeApps/ClaudeAppsView.vue` | 主界面：左侧应用列表 + 右侧对话面板 |
| `src/components/claudeApps/ClaudeAppsList.vue` | 应用列表组件，展示应用状态和操作按钮 |
| `src/components/DynamicFunctionHost.vue` | 应用运行宿主，管理 iframe、Bridge 注入、后端启停 |
| `src/components/DynamicFunctionCreator.vue` | 应用管理器，发现/刷新/同步桌面图标 |
| `src/stores/dynamicFunctionStore.ts` | Pinia Store，管理应用列表和后端状态 |
| `src-tauri/src/dynamic_function.rs` | Rust 后端，应用 CRUD + Node.js 进程管理 |
