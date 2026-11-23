# Coworks

Coworks 是 Moonton Claude Desktop 内置的 AI 协作工作台，提供三种工作模式：Chat（纯对话）、Code（项目开发）、Cowork（任务协作），通过 Claude SDK Sidecar 进程与 Claude API 通信，支持流式输出、工具调用、文件附件和会话持续性。

## 核心概念

| 概念 | 说明 |
|------|------|
| Mode | 工作模式：`chat`（对话）、`code`（编程）、`cowork`（任务） |
| Session | 会话实例，每个 mode + contextId 组合对应一个独立的 Sidecar 进程 |
| Sidecar | Node.js 子进程，封装 Claude SDK 调用，通过 stdin/stdout 与 Rust 后端通信 |
| SDK Session ID | Claude SDK 的持久化会话标识，实现跨请求的对话连续性 |
| Task | Cowork 模式下的工作单元，包含消息历史、关联文件、工作日志 |
| Skill App | 来自插件市场的技能应用，可作为任务的 System Prompt 注入 |

## 功能入口

桌面上的 **Cowork** 图标打开协作工作台。界面由左侧窄边栏（模式切换）和主内容区组成。

三种模式通过侧边栏切换：

| 模式 | 图标 | 用途 |
|------|------|------|
| Chat | 💬 | 通用对话，无工作目录限制 |
| Code | 💻 | 项目开发，绑定项目目录，自带文件树 |
| Cowork | 📁 | 任务协作，绑定工作目录，跟踪关联文件和工作日志 |

模式切换时保留各自的上下文状态，不会丢失已有的会话。

## Chat 模式

纯对话模式，适合问答、分析、翻译等不依赖特定目录的任务。

### 界面布局

```
┌──────────┬──────────────────────────────────────┐
│ 侧边栏   │  左面板          │  主面板            │
│ ┌──────┐ │  ┌────────────┐ │  ┌──────────────┐  │
│ │ Chat │ │  │ + 新建对话  │ │  │  消息列表    │  │
│ │ Code │ │  ├────────────┤ │  │  (流式输出)  │  │
│ │Cowork│ │  │ 历史对话    │ │  │              │  │
│ └──────┘ │  │  对话1      │ │  ├──────────────┤  │
│          │  │  对话2      │ │  │ 模型选择     │  │
│          │  │  ...        │ │  │ 输入框+附件  │  │
│          │  └────────────┘ │  └──────────────┘  │
└──────────┴──────────────────────────────────────┘
```

### 功能特性

- **多对话管理**：创建、切换、删除对话，自动根据首条消息生成标题
- **历史持久化**：对话保存到 `~/.MoontonClaudeDesktop/coworks/chats/`
- **会话连续性**：通过 `sdkSessionId` 实现跨请求的上下文保持
- **文件附件**：拖拽或点击添加文件（最多 5 个），随消息发送给 Claude

## Code 模式

项目开发模式，绑定本地项目目录，Claude 在该目录上下文中工作。

### 界面布局

```
┌──────────┬──────────────────────────────────────────────────┐
│ 侧边栏   │  左面板          │  主面板         │  右面板      │
│          │  ┌────────────┐ │  ┌───────────┐  │ ┌─────────┐ │
│          │  │ 打开项目... │ │  │ 消息列表  │  │ │ 文件树  │ │
│          │  ├────────────┤ │  │           │  │ │ (实时)  │ │
│          │  │ 最近项目    │ │  │           │  │ │         │ │
│          │  │  项目1      │ │  ├───────────┤  │ │ 上下文  │ │
│          │  │  项目2      │ │  │ 输入框    │  │ │  面板   │ │
│          │  └────────────┘ │  └───────────┘  │ └─────────┘ │
└──────────┴──────────────────────────────────────────────────┘
```

### 功能特性

- **项目选择**：通过系统文件选择器打开项目目录
- **最近项目**：记录最近使用的 10 个项目
- **文件树浏览**：展示项目目录结构，双击用系统默认应用打开文件
- **文件变更自动刷新**：Claude 执行 Write/Edit/Bash 等文件修改工具后，自动刷新文件树
- **上下文面板**：展示生成的文件信息，支持点击打开
- **会话持续性**：每个项目路径对应独立的 `sdkSessionId`，保存在 `~/.MoontonClaudeDesktop/coworks/projects/` 中

## Cowork 模式

任务协作模式，以"任务"为单位组织工作，支持关联文件跟踪和工作日志。

### 界面布局

```
┌──────────┬──────────────────────────────────────────────────┐
│ 侧边栏   │  左面板          │  主面板         │  右面板      │
│          │  ┌────────────┐ │  ┌───────────┐  │ ┌─────────┐ │
│          │  │ + 新建任务  │ │  │ 消息列表  │  │ │ 文件树  │ │
│          │  ├────────────┤ │  │           │  │ │         │ │
│          │  │ 活跃任务    │ │  │           │  │ │ 生成的  │ │
│          │  │  任务1      │ │  ├───────────┤  │ │  文件   │ │
│          │  │  任务2      │ │  │ 输入框    │  │ │         │ │
│          │  └────────────┘ │  └───────────┘  │ └─────────┘ │
└──────────┴──────────────────────────────────────────────────┘
```

### 功能特性

- **任务管理**：创建、切换、删除任务，每个任务绑定工作目录
- **任务状态**：`active` -> `completed` -> `archived`
- **关联文件**：自动追踪 Claude 读写的文件（read/write/edit/delete/move/copy）
- **工作日志**：记录 Claude 执行的操作摘要
- **Skill 集成**：创建任务时可选择已安装的 Skill App，其描述作为 System Prompt 注入
- **历史持久化**：任务保存到 `~/.MoontonClaudeDesktop/coworks/tasks/`

### 新建任务对话框

| 字段 | 必填 | 说明 |
|------|------|------|
| 任务名称 | 是 | 支持自动填充（从目录名） |
| 工作目录 | 是 | 通过文件选择器选取 |
| 技能应用 | 否 | 从已安装的插件技能中选择 |

## Sidecar 架构

Coworks 通过 Node.js Sidecar 进程与 Claude SDK 交互，每个 Session 对应一个独立的 Sidecar 实例。

### 通信协议

```
┌──────────────┐    stdin (JSON)     ┌──────────────────┐
│   Frontend   │ ──────────────────> │   Sidecar        │
│  (Vue)       │                     │   (Node.js)      │
│              │ <────────────────── │                   │
│              │    stdout (JSON)    │   Claude SDK      │
└──────────────┘                     └──────────────────┘
       ↑                                     │
       │  Tauri Events                       │ API
       │  coworks-sidecar-output-{id}        │
       │  coworks-sidecar-ready-{id}         ▼
       │  coworks-sidecar-stderr-{id}  ┌──────────────┐
       │                               │  Claude API  │
       └───── Rust (coworks_sidecar.rs) └──────────────┘
```

### 前端 -> Sidecar 消息

#### start

发起一次对话请求：

```json
{
  "type": "start",
  "sessionId": "msg-123",
  "workDir": "/path/to/project",
  "model": "auto",
  "prompt": "用户输入的内容",
  "mode": "code",
  "plugin": { "path": "...", "skillName": "...", "skillDescription": "..." },
  "attachments": [{ "path": "/file.png", "name": "file.png" }],
  "sdkSessionId": "sdk-session-abc",
  "systemPrompt": "自定义系统提示"
}
```

| 字段 | 说明 |
|------|------|
| `sessionId` | 前端消息 ID，用于关联流式输出 |
| `workDir` | 工作目录（Chat 模式为空） |
| `model` | 模型名称或 `auto` |
| `mode` | 工作模式 |
| `plugin` | 技能信息（Cowork 模式，可选） |
| `attachments` | 文件附件列表（可选） |
| `sdkSessionId` | SDK 会话 ID，用于恢复上下文（可选） |
| `systemPrompt` | 自定义系统提示（claudeapps 模式使用，可选） |

#### cancel

取消当前请求：

```json
{
  "type": "cancel",
  "sessionId": "msg-123"
}
```

#### close

关闭 Sidecar 进程：

```json
{
  "type": "close"
}
```

### Sidecar -> 前端消息

| 类型 | 字段 | 说明 |
|------|------|------|
| `ready` | - | Sidecar 初始化完成 |
| `session_started` | `sdkSessionId` | SDK 会话已建立，返回会话 ID |
| `text_delta` | `sessionId`, `content` | 文本增量（流式） |
| `tool_use` | `sessionId`, `toolName`, `toolId`, `input` | Claude 开始使用工具 |
| `tool_result` | `sessionId`, `toolId`, `summary` | 工具执行结果 |
| `done` | `sessionId` | 请求完成 |
| `error` | `sessionId`, `message` | 请求出错 |
| `cancelled` | `sessionId` | 请求已取消 |
| `pong` | - | 心跳响应 |

### 进程管理

- **启动**：前端首次发送消息时自动 spawn Sidecar
- **环境变量**：从 settingsStore 注入用户配置的环境变量（包括 `ANTHROPIC_AUTH_TOKEN`）
- **PATH 注入**：自动添加 npm global bin 目录（提供 `claude` 命令）和 Node.js 目录
- **隐藏窗口**：Windows 下使用 `CREATE_NO_WINDOW` 标志
- **关闭**：窗口关闭时发送 `close` 消息并 kill 进程
- **全量清理**：应用卸载时通过 `closeAllSessions()` 终止所有 Sidecar

### Sidecar 资源路径

| 环境 | 路径 |
|------|------|
| 生产 | `{resources}/coworks-sidecar/index.js` |
| 开发 | `src-tauri/resources/coworks-sidecar/index.js` |
| 开发 (备选) | `sidecar/coworks/dist/index.js` |

## 模型选择

ChatPanel 内置模型选择器，支持以下模型：

| 模型标识 | 说明 |
|----------|------|
| `auto` | 自动选择（默认） |
| 其他 | 由 settingsStore 中的 LLM Router 配置决定 |

模型设置会影响 Sidecar 发送给 Claude SDK 的 `model` 参数。

## 文件附件

ChatPanel 支持通过两种方式添加文件附件：

1. **拖拽**：将文件拖拽到输入区域
2. **点击**：点击附件按钮选择文件

限制：
- 最多 5 个附件
- 附件以 `{ path, name }` 格式随消息发送给 Sidecar

## 内置应用 (Builtin Apps)

Cowork 模式预置了以下内置应用模板：

| 应用 | 图标 | 用途 |
|------|------|------|
| 文件整理 | 📁 | 智能整理和归类文件 |
| 代码审查 | 🔍 | 审查和改进代码质量 |
| 数据分析 | 📊 | 分析和可视化数据 |
| 翻译助手 | 🌐 | 多语言翻译和本地化 |

此外，从插件市场安装的 Skill 也会作为可选应用出现在新建任务对话框中。

## 数据持久化

### 目录结构

```
~/.MoontonClaudeDesktop/coworks/
├── config.json                    # 全局配置（默认模型、上次模式）
├── recent-code.json               # Code 模式最近项目
├── recent-cowork.json             # Cowork 模式最近工作目录
├── chats/                         # Chat 模式对话
│   ├── index.json                 # 对话索引
│   └── {chat-id}.json             # 单条对话（含完整消息）
├── tasks/                         # Cowork 模式任务
│   ├── index.json                 # 任务索引
│   └── {task-id}.json             # 单个任务（含消息、关联文件、工作日志）
├── projects/                      # Code 模式项目会话
│   └── {path-hash}.json           # 项目 SDK Session ID 映射
└── claude-apps-chats/             # Claude Apps 对话（独立存储）
    ├── index.json
    └── {conversation-id}.json
```

### 索引 + 数据分离

所有数据类型（Chat、Task）采用相同的存储模式：

1. **索引文件** (`index.json`)：轻量元数据数组，用于列表展示，按 `updatedAt` 降序排列
2. **数据文件** (`{id}.json`)：完整数据，包含消息历史等大体积内容，按需加载

保存时同步更新索引和数据文件，删除时同步清理。

### 数据类型

#### ChatHistory

```typescript
interface ChatHistory {
  id: string
  title: string           // 自动从首条用户消息生成（截断 50 字符）
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  sdkSessionId?: string   // SDK 会话 ID
}
```

#### CoworksTask

```typescript
interface CoworksTask {
  id: string
  name: string
  appId?: string           // 关联的内置/技能应用
  workDir: string
  messages: ChatMessage[]
  relatedFiles: RelatedFile[]  // { path, operation, timestamp }
  workLogs: WorkLog[]          // { id, action, timestamp, details }
  createdAt: number
  updatedAt: number
  status: 'active' | 'completed' | 'archived'
  skillInfo?: TaskSkillInfo    // 技能信息（System Prompt 来源）
  sdkSessionId?: string
}
```

#### CodeProjectSession

```typescript
interface CodeProjectSession {
  projectPath: string
  sdkSessionId: string
  updatedAt: number
}
```

项目路径通过 `DefaultHasher` 哈希为文件名，避免路径中的特殊字符问题。

## 架构总览

```
┌──────────────────────────────────────────────────────────────┐
│                      CoworksApp.vue                           │
│  ┌──────────┐  ┌──────────────────────────────────────────┐  │
│  │ModeSwitch│  │ ChatModeView / CodeModeView / CoworkMode │  │
│  │ Chat     │  │   ┌──────────────┐  ┌────────────────┐   │  │
│  │ Code     │  │   │ ChatPanel    │  │ FileTree       │   │  │
│  │ Cowork   │  │   │  ChatMessage │  │ ContextPanel   │   │  │
│  └──────────┘  │   │  ToolUsage   │  │ TaskHistoryList│   │  │
│                │   │  ModelSelect │  │ NewTaskDialog  │   │  │
│                │   └──────┬───────┘  └────────────────┘   │  │
│                └──────────┼───────────────────────────────┘  │
└───────────────────────────┼──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                 Composables (Facade 层)                       │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  useChat.ts  (总入口 Facade)                         │    │
│  │  ├── mode / model / contextId / workDir              │    │
│  │  ├── Chat: createNewChat / loadChat / deleteChat     │    │
│  │  ├── Code: setProject                                │    │
│  │  └── Cowork: createNewTask / loadTask / deleteTask   │    │
│  └──────────────────────┬───────────────────────────────┘    │
│                         │ 组合                                │
│  ┌──────────┐  ┌───────┴─────┐  ┌───────────┐  ┌────────┐  │
│  │useSession│  │useChatHist. │  │ useTasks   │  │useWork │  │
│  │ Sidecar  │  │ Chat 持久化 │  │ Task 持久化│  │ space  │  │
│  │ 会话管理 │  │             │  │            │  │最近项目│  │
│  └────┬─────┘  └─────────────┘  └────────────┘  └────────┘  │
└───────┼──────────────────────────────────────────────────────┘
        │ Tauri invoke
        ▼
┌──────────────────────────────────────────────────────────────┐
│                    Rust 后端                                   │
│  ┌─────────────────────┐  ┌──────────────────────────────┐   │
│  │ coworks_sidecar.rs  │  │ coworks_config.rs            │   │
│  │ Sidecar 进程管理     │  │ 数据持久化 (Chat/Task/      │   │
│  │ spawn / send / kill │  │   Project/Workspace/Config)  │   │
│  └─────────┬───────────┘  └──────────────────────────────┘   │
│            │                                                  │
│  ┌─────────┴───────────┐  ┌──────────────────────────────┐   │
│  │ Node.js Sidecar     │  │ coworks_fs.rs                │   │
│  │ (index.js)          │  │ 文件系统操作（目录读取等）    │   │
│  │ Claude SDK 调用      │  │                              │   │
│  └─────────────────────┘  └──────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

## 关键源文件

### 前端

| 文件 | 职责 |
|------|------|
| `src/components/coworks/CoworksApp.vue` | 主容器：侧边栏 + 模式切换 + 视图路由 |
| `src/components/coworks/ModeSwitch.vue` | 三模式切换按钮组 |
| `src/components/coworks/views/ChatModeView.vue` | Chat 模式视图：对话列表 + 聊天面板 |
| `src/components/coworks/views/CodeModeView.vue` | Code 模式视图：项目列表 + 聊天面板 + 文件树 |
| `src/components/coworks/views/CoworkModeView.vue` | Cowork 模式视图：任务列表 + 聊天面板 + 文件树 |
| `src/components/coworks/ChatPanel.vue` | 聊天面板：消息展示、模型选择、输入框、文件拖拽附件 |
| `src/components/coworks/ChatMessage.vue` | 单条消息渲染（Markdown + 工具调用） |
| `src/components/coworks/ToolUsage.vue` | 工具调用展示（名称、输入、结果摘要、状态） |
| `src/components/coworks/ModelSelect.vue` | 模型选择下拉框 |
| `src/components/coworks/FileTree.vue` | 文件树组件（展开/折叠、选择、双击打开） |
| `src/components/coworks/ContextPanel.vue` | 上下文面板（生成文件预览） |
| `src/components/coworks/ChatHistoryList.vue` | Chat 模式对话历史列表 |
| `src/components/coworks/TaskHistoryList.vue` | Cowork 模式任务历史列表 |
| `src/components/coworks/NewTaskDialog.vue` | 新建任务对话框（名称 + 目录 + 技能选择） |
| `src/components/coworks/data/apps.ts` | 内置应用定义（文件整理、代码审查、数据分析、翻译助手） |

### Composables

| 文件 | 职责 |
|------|------|
| `src/composables/coworks/useChat.ts` | Facade composable，统一编排三种模式的对话逻辑 |
| `src/composables/coworks/useSession.ts` | 核心 Session 管理：Sidecar 通信、流式输出处理、事件监听 |
| `src/composables/coworks/useChatHistory.ts` | Chat 模式对话持久化（CRUD + 索引管理） |
| `src/composables/coworks/useTasks.ts` | Cowork 模式任务持久化（CRUD + 关联文件 + 工作日志） |
| `src/composables/coworks/useWorkspace.ts` | 工作空间管理（最近项目/目录、Code 模式 SDK Session） |
| `src/composables/coworks/useFileTree.ts` | 文件树数据管理（目录读取、展开/折叠） |
| `src/composables/coworks/useApps.ts` | 应用管理（内置应用 + 插件技能应用发现） |

### 后端 (Rust)

| 文件 | 职责 |
|------|------|
| `src-tauri/src/coworks_sidecar.rs` | Sidecar 进程管理器：spawn、stdin 写入、kill、stdout/stderr 事件转发 |
| `src-tauri/src/coworks_config.rs` | 数据持久化：Chat 历史、Task、项目会话、工作空间、配置 |
| `src-tauri/src/coworks_fs.rs` | 文件系统操作：目录读取、文件信息 |

### 类型定义

| 文件 | 职责 |
|------|------|
| `src/types/coworks.ts` | 运行时类型：AppMode、ChatMessage、ToolUsage、SidecarOutput |
| `src/types/coworks-persistence.ts` | 持久化类型：ChatHistory、CoworksTask、BuiltinApp、SkillApp、TaskSkillInfo |
