# /sync-progress

> **Invoked by:** agent-session or user-session

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 对比 session-log、session-state.json、milestone 文件和 wiki 数据。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Re-synchronize session-state.json and session-log.md with current workspace state.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

1. Read all milestone files in .claude/milestones/.
2. Compare task statuses with session-state.json.
3. Update session-state.json to reflect actual file system state.
4. Call `@write-session-log` with `entry_type: "sync"`. The skill re-derives the Pending layer from `session-state.json` (surfacing open tasks, unresolved blockers, coverage failures as pending items), refreshes derived views, and enforces the 150-line limit. Do not append to `session-log.md` directly.
5. **Knowledge graph update**: if `.claude/wiki/graph.json` exists, run `python scripts/graph_update.py` (incremental update — only re-processes changed files). Otherwise run `/build` or the full pipeline to generate it for the first time.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

Keep session-state.json and session-log.md aligned with actual outcomes and unresolved blockers.

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

session-state.json and session-log.md accurately reflect workspace state.
