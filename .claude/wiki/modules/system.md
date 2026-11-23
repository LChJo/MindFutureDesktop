# system 模块文档

> 此文件由 wiki_gen 生成。手动编辑将在重新生成时保留。



> 关联文件: `src-tauri/src/nodejs.rs`, `src-tauri/src/git.rs`, `src-tauri/src/uv.rs`, `src-tauri/src/system_aliases.rs`, `src-tauri/src/updater.rs`, `src/components/SetupGuide.vue`, `src/components/SetupGuideWindow.vue`, `src/components/UpdateModal.vue`, `src/composables/useSetupGuide.ts`, `src/composables/useAutoUpdate.ts`

## 职责

System dependency management (Node.js, Git, UV), auto-update mechanism, first-run setup guide, and system alias resolution

## 所属路径

- src-tauri/src/nodejs.rs
- src-tauri/src/git.rs
- src-tauri/src/uv.rs
- src-tauri/src/system_aliases.rs
- src-tauri/src/updater.rs
- src/components/SetupGuide.vue
- src/components/SetupGuideWindow.vue
- src/components/UpdateModal.vue
- src/composables/useSetupGuide.ts
- src/composables/useAutoUpdate.ts

## 实体

### SystemDependency (entity)

External system dependency (Node.js, Git, UV)

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |


## 接口

### checkDependencies `[入站]`

Check all system dependencies

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  SystemDependency[]
  ```

### checkForUpdate `[入站]`

Check for application updates

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  {}
  ```
- **输出**:
  ```ts
  { available, version, url }
  ```

### installUpdate `[入站]`

Download and install update

| 来源 | 位置 |
|------------|--------|
| DECLARED |  |

- **输入**:
  ```ts
  { url }
  ```
- **输出**:
  ```ts
  void
  ```


## 场景

### First-run setup guide

1. App detects missing dependencies
2. Setup guide wizard opens
3. User installs missing dependencies
4. All checks pass, guide closes

**边界情况**:
- Partial installation
- Permission denied
- Network offline

### Auto-update check

1. App starts or periodic check fires
2. Check GitHub releases for new version
3. Show update modal if available
4. User confirms and installs

**边界情况**:
- Network timeout
- Download interrupted
- Version downgrade attempt


## 约束

### Performance

- Dependency check (all three) < 2s total
- Update download uses streaming with progress reporting

### Security

- Update packages verified against expected checksums
- Node.js bundled binary signed (macOS)

### Data

- Dependency paths cached after first resolution, invalidated on system PATH change
- Update check interval: once per app launch, with 24h cooldown


## 设计决策

- **Bundled Node.js for macOS**
  - 理由: Ensures consistent Node.js version for sidecar processes
  - 备选方案: System Node.js only, WASM-based runtime

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
