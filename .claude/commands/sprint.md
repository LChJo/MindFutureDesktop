# /sprint

> **Invoked by:** user-session Calls agent: `lead`

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Dispatch the Lead agent in sprint mode to execute all unblocked tasks in the current milestone.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

1. ### Phase 1: Wave Planning
2. Read session-state.json and session-log to identify the current milestone, task status, and workspace_readiness.
3. Determine sprint phase: if not execution_ready → PREPARATION (only prep tasks). If execution_ready → EXECUTION (all tasks).
4. Invoke Lead agent for wave planning: Lead reads state, computes waves, returns structured JSON dispatch plan.
5. Parse the dispatch plan: extract wave list and task prompts.
6. ### Phase 2: Execution Loop
7. Wave execution loop: per wave, dispatch module agents in parallel by issuing ONE single message containing multiple foreground `Agent` tool calls (do NOT set `run_in_background: true`). Parallel sub-agents must run synchronously so all results return in the same turn — a backgrounded agent breaks wave coordination and delays the next wave. After every agent in the wave returns, collect results and update session-state.json before starting the next wave.
8. After all prep tasks DONE: re-derive readiness. If blockers resolved → promote to execution_ready.
9. Update session-state.json with task outcomes; sync session-log via `@write-session-log` (one call per task outcome, entry_type: "sprint").
10. ### Phase 3: Post-Sprint
11. Post-Sprint: GC scan → Auto-Fix (max 2 rounds) → Regression Verify → Milestone Gate.
12. Coverage Gate Check: Read session-state.json milestone.coverage_gate.status. If 'pending' and all tasks DONE → run /sprint-review to evaluate coverage. If 'failed' → HALT advancement, set continuation.paused_reason to 'coverage_gate_failed', report which modules need more test coverage. Do NOT advance to the next milestone. **NEVER lower coverage_gate.target or skip verification to bypass a failed gate** — the only paths forward are: fix the tests, fix the environment, or get explicit user consent to skip.
13. Auto-Advance: if milestone gate passes AND coverage_gate.status is 'passed', advance current_milestone to next and continue. If the milestone has NO TDD specs at all, do NOT auto-advance — use AskUserQuestion to confirm with the user: 'This milestone has no TDD specs. Advance without test coverage?' Options: 'Yes, advance' / 'No, create TDD specs first'. Only advance with explicit user consent; record the decision via `@write-session-log` (entry_type: "sprint", evidence includes the user's consent text).

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

保持 session-state.json 与实际结果对齐；session-log.md 通过 `@write-session-log` 同步（未解决的阻塞反映在 Pending 层）。



## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

All milestone tasks DONE with verification evidence, or milestone gate pause.
