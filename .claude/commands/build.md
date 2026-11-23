# /build

> **Invoked by:** agent-session (module agents)

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Implement the plan from /plan and deliver working code that satisfies the task.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

1. Re-read the plan output and confirm scope.
2. Write code that stays within declared write_paths.
3. Write or update tests matching the TDD spec.
4. Run verification commands after each logical change.
5. Call @write-session-log with entry_type: "build" to record progress notes (do not append to session-log.md directly).

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

保持支持文档对齐；session-log.md 通过 `@write-session-log` 与当前 task 状态同步。



## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

All acceptance criteria implemented and local verification passes.
