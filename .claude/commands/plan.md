# /plan

> **Invoked by:** agent-session (lead or module agents) | user-session (with `--dispatch`)

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Lock in the smallest correct implementation plan before code changes begin.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

### Step 0 — Session Detection & Dispatch Assessment

**Sprint 守卫：** 如果 `.claude/progress/sprint-context.md` 存在且 `session-state.json` → `continuation.phase` == `"sprint"` → 跳过分派，直接执行 Step 1-7。

**用户直接调用时：**

1. 解析 `$ARGUMENTS`：提取 `--dispatch` flag 和任务/范围描述 `$SCOPE_DESCRIPTION`。
2. 如果 `--dispatch` 存在：
   a. 读 `.claude/wiki/modules/` 构建 module-slug → paths 映射表。
   b. 将 `$SCOPE_DESCRIPTION` 中涉及的路径/功能域匹配到模块所有权。
   c. 如果匹配到单个 `core-{module}` agent：

      展示 Dispatch Proposal：

      ```
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
       DISPATCH PROPOSAL
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

      Module: $MODULE_NAME
      Agent: core-$MODULE_SLUG
      Scope: $SCOPE_DESCRIPTION
      Write paths: $WRITE_PATHS
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
      ```

      使用 AskUserQuestion 确认：
      - **"确认 dispatch"** → dispatch 给 `core-{module}` agent 执行 `/plan`，等待返回结果后展示给用户 → EXIT。
      - **"继续本地处理"** → 跳过 dispatch，继续 Step 1。

   d. 如果匹配多个模块 → 显示模块列表，让用户选择目标模块或本地处理。
   e. 如果无匹配 → 继续 Step 1。
3. 如果无 `--dispatch` → 继续 Step 1。

### Step 1 — Read session-log and the active milestone task file.

### Step 2 — Read the relevant rule files and wiki module docs.

### Step 3 — Challenge scope: name the smallest change that satisfies the task.

Example: if task says "add user auth", the smallest scope is login endpoint + JWT validation — not a full RBAC system.

### Step 4 — Describe architecture, interfaces, failure modes, and dependency boundaries.

Example: "New POST /login endpoint in api module → calls domain.authenticateUser → returns JWT. Failure: invalid credentials → 401."

### Step 5 — Map every acceptance criterion to verification and update the TDD spec.

Example: acceptance "user can login" → test case "POST /login with valid creds returns 200 + token".

### Step 6 — List only the unresolved decisions that cannot be derived from the repository.

### Step 7 — Do not write code in this step.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

If the task has non-trivial code paths, update the relevant test spec in `.claude/tdd/`.

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

A concrete implementation plan exists with scope, architecture, verification, and open decisions made explicit.
