# desktop 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src/components/Desktop.vue`, `src/components/WindowManager.vue`, `src/components/Window.vue`, `src/components/Taskbar.vue`, `src/components/DesktopIcon.vue`, `src/components/WidgetPanel.vue`, `src/components/FileManager.vue`, `src/stores/desktopStore.ts`, `src/stores/windowStore.ts`, `src/stores/fileSystemStore.ts`

## 职责

Virtual desktop environment with icon management, multi-window system, taskbar, widget panel and virtual file manager

## 所属路径

- src/components/Desktop.vue
- src/components/WindowManager.vue
- src/components/Window.vue
- src/components/Taskbar.vue
- src/components/DesktopIcon.vue
- src/components/WidgetPanel.vue
- src/components/FileManager.vue
- src/stores/desktopStore.ts
- src/stores/windowStore.ts
- src/stores/fileSystemStore.ts

## 实体

### DesktopIcon (entity)

Draggable desktop icon representing an app or shortcut

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### Window (entity)

Managed window with title bar, resize, minimize, maximize, close

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

### FileNode (entity)

Virtual file system node for the desktop file manager

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### openWindow `[入站]`

Open a new window for the given component

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  openWindow(icon: DesktopIcon, windowData?: Record<string, unknown>)
  ```
- **输出**:
  ```ts
  Promise<void>  // 重复图标时聚焦已有窗口
  ```
- **错误码**: `COMPONENT_LOAD_FAILED`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_

### closeWindow `[入站]`

Close and destroy a window by ID

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { windowId }
  ```
- **输出**:
  ```ts
  void
  ```
- **错误码**: `WINDOW_NOT_FOUND`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_

### focusWindow `[入站]`

Bring window to front and set focus

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { windowId }
  ```
- **输出**:
  ```ts
  void
  ```

### moveIcon `[入站]`

Drag-drop repositioning of desktop icon

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { iconId, newPosition }
  ```
- **输出**:
  ```ts
  void
  ```
- **错误码**: `ICON_NOT_FOUND`
  - _错误码按 `architecture/overview.md` → 错误策略声明的方式抛出（默认 `throw Error`）。_


## 场景

### User opens an app from desktop icon

1. Double-click desktop icon
2. WindowManager creates new Window
3. Component rendered inside window
4. Window gets focus and top z-index

**边界情况**:
- Window already open for same app
- Maximum window count reached

### User rearranges desktop icons

1. Drag icon to new position
2. Position saved to store
3. Layout persisted

**边界情况**:
- Overlapping icons
- Icon dragged off-screen

### User browses virtual file manager

1. Open FileManager window
2. Navigate folders via breadcrumb or double-click
3. Select, copy, paste, rename files

**边界情况**:
- Empty folder
- Deep nesting


## 约束

### Performance

- Window operations < 16ms to maintain 60fps


## 设计决策

- **Pinia store for window/icon state**
  - 理由: Centralized state management consistent with Vue 3 ecosystem
  - 备选方案: provide/inject, event bus

## 相关任务

| ID | 标题 | 阶段 | 状态 | 里程碑 |
|---|---|---|---|---|
| T1-2 | Add unit tests for Pinia stores | exec | NOT_STARTED | M1 |

## 错误处理

- 方式: Result/Error propagation in Rust, try-catch in TypeScript
- 重试策略: no automatic retry for user-facing operations
- 降级行为: graceful degradation with user notification



## 相关依赖模块

- [claude-bot](claude-bot.md) — Autonomous Claude Bot engine with multi-channel integration 
- [coworks](coworks.md) — AI-powered collaboration hub with three modes: Chat (convers

## 相关社区

_(无图社区)_
