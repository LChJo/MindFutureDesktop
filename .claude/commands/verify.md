# /verify

> **Invoked by:** agent-session (module agents) or user-session

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Verify code changes — auto-detects PBVF task context vs standalone review.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

### Step 0 — Intent Detection

Read `.claude/progress/session-state.json`. Check if any task has `status: "IN_PROGRESS"` with `pbvf_phase` in `{"build", "verify", "fix"}`. If YES → follow **PBVF Mode** below. If NO (or session-state.json does not exist) → follow **Standalone Mode** below.

### PBVF Mode (Sprint / Task Context)

### Step 1 — Locate the current task

Call `@read-task`: read `session-state.json` for the active `task_id`, then read the milestone file to extract **acceptance criteria**, **write_paths**, and **tdd_link**.

### Step 2 — Run verification commands

Run `verification_commands` from `project.json` (delegate to `@run-verify`).

### Step X.1 — Write-code structural self-check (always runs, pure Python)

Before acceptance review, run the structural write-check gate. It is pure Python (no LLM) and catches the four classes of issue covered by `@write-code-check`:

```bash
python <SKILL_SCRIPTS>/write_code_check.py \
    --task <task_id> \
    --root <project_root> \
    --changed-files <file1> <file2> ...
```

Exit semantics:

- Exit **0** → no blocking findings; continue to Step 3.
- Exit **1** → at least one **high**-severity finding. HALT the verify flow and address the findings listed in the report before proceeding.
- Report is written to `.claude/progress/write-check-{task_id}-{YYYYMMDD-HHMMSS}.md` (a legacy `write-check-{task_id}.md` copy is kept for backward-compatible tooling).

If you intentionally want to accept the findings (e.g. a flagged boundary is tracked as tech-debt for a follow-up task), re-run with:

```bash
python <SKILL_SCRIPTS>/write_code_check.py \
    --task <task_id> --root <project_root> \
    --changed-files ... \
    --accept-check "<one-line reason>"
```

This flips the exit code back to 0 AND appends an entry under the Pending layer of `session-log.md`:

```
- **acknowledged: <reason>** (write-check <task_id>, high findings count=<N>)
```

Never use `--accept-check` to bypass genuine violations silently — the reason is recorded for later review by `/sprint-review` and the reviewer pending-high scan.

### Step X.2 — Semantic self-check (only with `--deep`, fill-* pattern)

When the sprint policy or a manual run requests a deep review, add `--deep`:

```bash
python <SKILL_SCRIPTS>/write_code_check.py \
    --task <task_id> --root <project_root> \
    --changed-files ... \
    --deep
```

`--deep` runs the T8-7 semantic layer via the `fill-*` pattern. No LLM API call happens inside Python — instead the script produces an instruction file for the current Claude Code session to fill. The complete dance is:

1. **Emit.** First `--deep` run writes `.claude/progress/rule-violation-{task_id}-pending.md` containing:
   - the review instruction (代码规则审查者),
   - the content of every `.claude/rules/*.md`,
   - pointers to each changed file,
   - a YAML schema, and
   - a `<fill-findings>` placeholder.

   When only a pending emit happens and there are no structural highs, exit code is **0** — the run is **not** blocking, it is waiting for the fill.

2. **Fill.** Open the pending file, read the Context and Schema sections, and replace the `<fill-findings>` placeholder with a YAML block listing each rule violation found (or `findings: []` when none).

3. **Re-run.** Execute the same `--deep` command again. The script detects the filled placeholder, schema-validates each finding via `core.llm_instruction.read_filled_findings`, and merges them into the write-check report under `semantic_findings:`.

4. **Merge.** The combined (structural + semantic) `high` count decides the exit code:
   - combined high > 0 → exit 1 (unless overridden by `--accept-check`),
   - combined high == 0 → exit 0.

Non-`--deep` runs never touch the semantic layer, so they never consume any LLM tokens.

If the script prints:

```
[write-check] Semantic findings pending. Open and fill:
  .claude/progress/rule-violation-{task_id}-pending.md
Then re-run: python write_code_check.py --task {task_id} --root {root} --deep
```

do not skip the fill — act on it before marking the task DONE. If the pending file shows `findings: []` after your review, that is the correct filled state for "no violations".

### Step 3 — Acceptance criteria check

For each acceptance criterion, determine PASS or FAIL with concrete evidence (cite `file:line`, command output, or test result). Produce a checklist:

- `- [x] criterion — evidence`
- `- [ ] criterion — reason`

### Step 4 — Identify changed files (layered fallback)

(a) `git diff --name-only <start-commit> HEAD`
(b) `git diff --cached --name-only`
(c) `git diff --name-only`

Merge and deduplicate. If all empty, recall the files modified during this session from conversation context (build phase outputs, tool call history). Cross-check against the task's `write_paths` — flag any file outside `write_paths` as a **BOUNDARY_VIOLATION**.

### Step 5 — Focused code review

Review only the changed files using the review heuristics (adaptive depth by diff size, confidence gate, hard exclusions). Focus on logic correctness, boundary respect, and regression risk.

### Step 6 — TDD spec check

If `tdd_link` exists, read `.claude/tdd/specs/{module}.spec.md`, confirm that test cases relevant to this task's acceptance criteria are checked (`- [x]`). Note unchecked cases.

### Step 7 — Record evidence in session-state.json

Update `test_snapshot`, acceptance results, and any boundary violations.

### Step 8 — Decide DONE or /fix

If any acceptance criterion FAILED or `verification_commands` FAILED, or write-code-check Step X.1 exited 1 without `--accept-check`: do not mark DONE; proceed to `/fix`.

### Standalone Mode (Manual Review)

### Step 9 — Identify recent changes

Run `git diff --name-only HEAD~5 HEAD` and `git diff --cached --name-only` to list recently modified and staged files. If the user provided a specific scope (file or directory), narrow to that.

### Step 10 — Focused code review

Review the changed files using the review heuristics. Focus on logic correctness, edge cases, security issues, and error handling. Produce findings with confidence scores.

### Step 11 — Test planning

For each changed module, recommend which tests to write or update. If `.claude/tdd/specs/` exists, check whether existing TDD spec test cases cover the changes — note gaps.

### Step 12 — Run verification commands

If `project.json` exists and contains `verification_commands`, run them and report PASS/FAIL. If no `project.json`, skip.

### Step 13 — Output a review report

Changed file summary, review findings (sorted by severity), test recommendations, and verification results.

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

在 session-state.json 中记录验证证据，并通过 `@write-session-log` 更新 session-log.md（逐条验收结果、变更文件列表、边界校验）。如果审查发现缺失的验证，更新 TDD spec。

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

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

PBVF: every acceptance criterion verified with traceable evidence, write-code-check Step X.1 exit 0 (or accepted via `--accept-check`), and — when `--deep` was invoked — the semantic pending file is filled and merged. Standalone: review report with findings and test recommendations.
