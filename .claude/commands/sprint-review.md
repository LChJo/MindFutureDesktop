# /sprint-review

> **Invoked by:** user-session Calls agent: `gc`

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Run TDD coverage gate, workspace hygiene scan, UI visual audit, and LLM-driven
wiki semantic review for the current milestone.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

1. Read session-state.json for current milestone, task list, and coverage_gate status.
2. ### Gate Entry Check
   If NOT all tasks are DONE → HALT: report which tasks are incomplete, do NOT
   proceed with coverage gate. If coverage_gate.status is 'passed' → skip to
   Workspace Hygiene (gate already cleared). If coverage_gate.status is
   'pending' OR 'failed' → proceed to TDD Coverage Gate (re-evaluation allowed
   for failed gates to give a remediation chance).
3. ### TDD Coverage Gate (Strict Execution)
   For each module in the current milestone: (a) Read the TDD spec from
   `.claude/tdd/specs/{module}.spec.md`. (b) Count total test case checkboxes
   (both `- [ ]` and `- [x]`). (c) **MANDATORY**: Run verification_commands
   from project.json — tests MUST actually execute and pass. Checkbox
   completion alone is NOT sufficient. (d) Compute coverage percentage =
   (checked cases / total cases) × 100. Delegate to `@run-coverage` for full
   execution including test environment recovery.
4. ### Test Execution Enforcement
   **NEVER** lower coverage_gate.target or skip verification to work around
   failures. If verification_commands fail or cannot execute: (a) Diagnose the
   root cause (missing deps, wrong runtime, port conflict, missing config).
   (b) Attempt auto-fix (install dependencies, copy template configs).
   (c) Re-run verification. (d) If still failing, use AskUserQuestion to ask
   the user how to set up the test environment. (e) Only skip with explicit
   user consent — record skip reason via `@write-session-log`
   (entry_type: "sprint", pending_add: a Pending item flagging the skipped
   verification) and set coverage_gate.status to 'failed'.
5. Record coverage results: Update session-state.json milestone.coverage_gate
   with `{status: 'passed'|'failed', actual: <percent>, report: '<summary>',
   checked_at: '<ISO timestamp>'}`. Gate passes ONLY when actual >= target AND
   all verification_commands pass.
6. ### Workspace Hygiene
   Invoke GC agent to scan for orphaned artifacts.
7. Review code quality across completed tasks.
8. ### UI Visual Audit
   If project has a UI (project_type is web-app, web-service, desktop-app, or
   mobile-app): run visual screenshot audit using `@run-ui-audit`. Capture
   screenshots of key pages/screens, compare against wiki specs and TDD
   scenarios, report layout issues, rendering anomalies, or missing content.
9. ### Wiki Semantic Review (LLM-driven, fill-* pattern)
   This step runs only during `/sprint-review`; `/verify` and `/build` never
   trigger it (cost control). The loop is **emit → fill → ingest**:

   1. **Emit pending files.** Run:
      ```bash
      python <SKILL_SCRIPTS>/reviewer.py --root <project_root> --emit-wiki-review
      ```
      This writes three files under `.claude/progress/`:
      - `wiki-coverage-pending.md` — PRD vs wiki coverage gaps
      - `wiki-ambiguity-pending.md` — interface signature clarity
      - `wiki-conflict-pending.md` — cross-module contradictions

   2. **Fill `<fill-findings>`.** Open each pending file in turn. Read the
      `## Instruction` section, consult every path listed under
      `## Context files`, and follow the `## Schema` block. Replace the
      `<fill-findings>` placeholder under `## Findings` with a YAML block
      shaped like:
      ```yaml
      findings:
        - severity: high | medium | low
          category: <fixed per file — coverage_gap / signature_ambiguity / cross_module_conflict>
          source_ref: <wiki path:Line or PRD §section>
          module: <module slug, optional>
          detail: <one-sentence problem description>
          suggested_fix: <one-sentence remediation>
      ```
      YAML only — no prose, no preamble. Wrap in a ```yaml fenced block. Cap
      at 20 findings per file, most important first. Emit `findings: []` if
      no issues.

   3. **Ingest.** Run:
      ```bash
      python <SKILL_SCRIPTS>/reviewer.py --root <project_root> --ingest-wiki-review
      ```
      - `high` severity findings become errors (exit non-zero, sprint blocked).
      - `medium` / `low` become warnings (non-blocking).
      - Filled pending files are archived to `.claude/progress/history/`.
      - Unfilled files are left in place with a warning — re-fill them and
        re-run ingest.
      - Schema errors are reported as warnings without crashing.

   4. **Remediate & repeat.** If any `high` findings were reported, apply the
      `suggested_fix` from each to the wiki, then restart this step from (1).
      Continue until ingest reports no `high` findings. `medium` / `low`
      findings are advisory and may be deferred.
10. If coverage_gate.status is 'failed' OR wiki semantic review has
    unresolved `high` findings: Report which modules are below target, what
    test cases remain unchecked, and which wiki findings still block.
    Do NOT clear the gate — the sprint cannot advance. Do NOT suggest
    lowering the target.
11. Report all findings and recommendations. If coverage gate passed AND UI
    audit has no critical issues AND wiki semantic review reports no `high`
    findings, note that `/sprint` can now auto-advance to the next milestone.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

保持 session-state.json 和 session-log.md 与实际结果和未解决的阻塞对齐。

## Review Heuristics

Review depth: `standard`

### Hard Exclusions (Auto-Discard)

- Pure formatting or style issues already covered by linter/formatter
- Import ordering changes
- Comment-only changes with no semantic impact
- Whitespace-only diffs

### Confidence Gate

Every reported finding must include a confidence score (1-10).
Only findings with confidence >= 7 should be flagged as actionable.
Findings with confidence 4-6 should be noted as observations.
Findings with confidence <= 3 should be discarded.

### Verification Requirements

- All acceptance criteria from the task must be verified with evidence
- No regressions in existing tests (run full verification_commands)
- Code follows declared architecture boundaries (check module write_paths)
- New public behavior has corresponding tests

### Workspace Hygiene

- Check for orphaned files (wiki pages, TDD specs, agent files not linked to tasks)
- Check for dead code (unused imports, unreachable branches)
- Check for documentation-code drift (wiki claims vs actual implementation)
- Check for test gaps (public APIs without test coverage)

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

Coverage gate evaluated (tests actually executed), hygiene scan complete,
UI audit recorded, and wiki semantic review has no unresolved `high` findings.
