# /amend

> **Invoked by:** user-session (during implementation when task changes are needed)

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Modify, insert, or delete tasks in the current milestone through a structured discussion-then-execute workflow. All changes must be discussed with the user and confirmed before any file is written.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

### Step 1 — Listen to Intent

Ask the user to describe what they want to change in one or two sentences. Do NOT present a fixed menu of change types. Do NOT ask for a TASK-ID upfront. Let the user speak freely.

### Step 2 — Read Context and Restate Understanding

1. Read `session-state.json` to understand current milestone and task statuses.
2. Read `session-log.md` for recent decisions and known issues.
3. Read the relevant milestone task file(s) to understand task structure and dependencies.
4. Restate your understanding of the requested change in natural language.
5. If there is ambiguity, ask **one** clarifying question (the most critical one). If no ambiguity, proceed directly to Step 3.

### Step 3 — Propose Complete Change Plan (DO NOT write files yet)

Present a complete change proposal including ALL of the following:

1. **Change summary** — what will be modified / inserted / deleted.
2. **Affected tasks** — for each task being changed, show the complete updated fields:
   - Title, description, acceptance criteria, assigned agent, dependencies, write_paths.
   - For new tasks: full task definition including ID assignment.
   - For deleted tasks: ID and reason for removal.
3. **Dependency chain impact** — which downstream tasks need dependency updates.
4. **Milestone impact** — whether the change affects milestone scope or timeline.
5. **Session-log call preview** — the exact `@write-session-log` invocation that will be issued (entry_type, summary, evidence fields, pending_add).

**Wait for explicit user confirmation before proceeding.** If the user requests modifications to the proposal, revise and present again.

### Step 4 — Execute Changes

Only after user confirmation:

1. **Re-read** the latest task file(s) and session-state.json (do NOT rely on cached content from Step 2).
2. Apply the confirmed changes to the milestone task file.
3. Synchronize dependency fields in all affected tasks (both upstream and downstream).
4. If tasks were added, add them to `session-state.json` with status `NOT_STARTED`.
5. If tasks were deleted, remove them from `session-state.json`.
6. Call `@write-session-log` with:
   - `entry_type`: `"amend"`
   - `summary`: one-line description of the amendment
   - `evidence`: timestamp, affected tasks (IDs + changed fields), dependency chain impact, milestone impact
   - `pending_add`: IDs of newly inserted tasks (they start as not-yet-done), plus any new blockers surfaced
   - `pending_resolve`: IDs removed or tasks the amendment completes
   Do not append to `session-log.md` directly.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

- Updated milestone task file with changes applied
- Updated `session-state.json` reflecting task inventory changes
- `session-log.md` updated **via `@write-session-log`** with amendment record, rationale, and pending entries for newly inserted tasks

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

All confirmed changes are applied to task files, session-state.json is consistent with the updated task inventory, and `@write-session-log` has recorded the amendment (with rationale and pending entries for newly inserted tasks).
