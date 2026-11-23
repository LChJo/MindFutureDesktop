# /fix

> **Invoked by:** agent-session (module agents) | user-session (with `--dispatch`)

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Diagnose the root cause and repair the failure found in /verify.

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

**Sprint 守卫：** 如果 `.claude/progress/sprint-context.md` 存在且 `session-state.json` → `continuation.phase` == `"sprint"` → 跳过分派，直接执行 Step 1-4。

**用户直接调用时：**

1. 解析 `$ARGUMENTS`：提取 `--dispatch` flag 和失败描述 `$FAILURE_DESCRIPTION`。
2. 如果 `--dispatch` 存在：
   a. 读 `session-state.json` 获取失败证据（或使用用户提供的错误信息）。
   b. 识别失败的文件/测试，读 `.claude/wiki/modules/` 映射到模块所有权。
   c. 如果匹配到单个 `core-{module}` agent：

      展示 Dispatch Proposal：

      ```
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
       DISPATCH PROPOSAL
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

      Module: $MODULE_NAME
      Agent: core-$MODULE_SLUG
      Failure: $FAILURE_DESCRIPTION
      Affected files: $FILE_LIST
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
      ```

      使用 AskUserQuestion 确认：
      - **"确认 dispatch"** → dispatch 给 `core-{module}` agent 执行 `/fix` + `/verify` 循环，等待结果后展示给用户 → EXIT。
      - **"继续本地处理"** → 跳过 dispatch，继续 Step 1。

   d. 如果所有权不明确 → 让用户确认目标模块或本地处理。
3. 如果无 `--dispatch` → 继续 Step 1。

### Step 1 — Read the failure evidence from session-state.json.

### Step 2 — Identify root cause (not just symptoms).

### Step 3 — Apply the minimal fix.

### Step 3.5 — Record the fix via `@write-session-log`.

Call `@write-session-log` with:

- `entry_type`: `"fix"`
- `summary`: one-line description of what was fixed
- `evidence`: root_cause, fix_summary, files_changed, referenced failure snapshot
- `pending_resolve`: the pending item(s) this fix cleared (the original verify failure)
- `pending_add`: any new pending items uncovered during the fix (e.g. "regression test TBD", follow-up refactor)

If the fix did **not** resolve the root cause, keep the pending item in place (do not pass it to `pending_resolve`) and note the attempt count in `evidence`.

### Step 4 — Return to /verify to confirm the fix.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

Preserve failure evidence trail **via `@write-session-log`** (Step 3.5) before returning to /verify.

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

The fix resolves the root cause and /verify now passes.
