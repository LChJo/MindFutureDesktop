# /wikiorganize

> **Invoked by:** user-session

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

Re-analyze the project (source code + documents) using graphify, refresh wiki pages with the latest graph data, and clean up orphaned wiki pages.

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

### Step 1 -- Read Context

1. Read `.claude/progress/session-state.json` for the current milestone.
2. Read `.claude/project.json` for project config. Note: since M3 the
   `modules` and `milestones` arrays are NOT stored inside `project.json`.
3. Locate modules. Priority order:
   - `.scratch/modules/` directory (split format, one JSON per module) -- **preferred**
   - `.scratch/modules.json` (single array file) -- fallback
   If neither exists, abort with a clear message: the user must declare
   modules (with `paths` pointing at source code) before wikiorganize can
   extract code into the graph.
4. Locate milestones similarly: `.scratch/milestones/` or `.scratch/milestones.json`.
5. Verify all milestone tasks are DONE (or confirm user intent to reorganize mid-milestone).

### Step 2 -- Reorganize Wiki

1. **Locate the skill scripts directory.**  `wiki_organize.py` ships with
   the harness skill, NOT the user's project, so its path is ABSOLUTE.
   Standard install locations (check existence in this order):

   - Windows: `%USERPROFILE%\.claude\skills\harnessframework\scripts\`
   - macOS/Linux: `$HOME/.claude/skills/harnessframework/scripts/`
   - Claude Code plugin dirs: `$CLAUDE_PROJECT_DIR/.claude/plugins/*/skills/harnessframework/scripts/`
     or similar plugin paths

   Verify `<SKILL_SCRIPTS>/wiki_organize.py` exists before running.

2. **Invoke the CLI** (note the absolute script path):
   ```
   # Windows (PowerShell)
   python "$env:USERPROFILE\.claude\skills\harnessframework\scripts\wiki_organize.py" `
       --root "<project_root>" `
       --project "<project_root>\.claude\project.json" `
       --modules "<project_root>\.scratch\modules" `
       --milestones "<project_root>\.scratch\milestones" `
       --force

   # macOS / Linux / git-bash
   python "$HOME/.claude/skills/harnessframework/scripts/wiki_organize.py" \
       --root "<project_root>" \
       --project "<project_root>/.claude/project.json" \
       --modules "<project_root>/.scratch/modules" \
       --milestones "<project_root>/.scratch/milestones" \
       --force
   ```
   `--modules` and `--milestones` accept either a single JSON file (array)
   or a directory of split JSONs.  The CLI itself does NOT rely on CWD —
   always use the project's absolute paths for `--root`, `--project`,
   `--modules`, `--milestones`.
2. The pipeline will:
   - **Preflight step** — auto-install tree-sitter grammar packages for every
     language detected in `modules[*].paths` (TypeScript, Go, Rust, Java,
     C#, C++).  Progress is printed to stderr as:
     ```
     [harness] Languages detected: TypeScript (.ts, .tsx)
     [harness] Ensuring tree-sitter grammars...
     [harness]   .ts: OK
     [harness]   .tsx: OK
     [harness] Preflight complete.
     ```
     Python-only projects skip this step (stdlib `ast` suffices).  Users
     never have to `pip install` anything manually.  If an install fails
     (e.g. no network), the pipeline logs a WARN and continues in
     degraded mode (doc-only fallback for the affected language).
   - Build a fresh knowledge graph from source code (via modules.paths)
     AND documents (`.claude/wiki/`).
   - Regenerate all wiki pages with the latest graph data.
   - Regenerate graph-wiki/ community articles and graph.json.
   - Remove orphaned graph-wiki pages no longer in the graph.
   - Run a wiki health check.
3. If the output reports `nodes: N` but all kinds are `concept`/`decision`/`document`
   (no `entity`/`interface`), code AST extraction was skipped. Check:
   - `modules[*].paths` actually point at source directories
   - The preflight step's stderr output — any `FAILED` lines indicate
     grammar packages that could not be installed (usually network issues)

### Step 3 -- Report Changes

Report a summary including:
- New and updated wiki pages.
- Removed orphan pages.
- Graph statistics (nodes, edges, communities).
- Any remaining health issues (dead links, placeholders, coverage gaps).

### Step 4 -- Record in Session Log

Append a wiki organize entry to `session-log.md`:

```markdown
### Wiki Organize ($DATE)
- **Refreshed pages:** $COUNT
- **Removed orphans:** $ORPHAN_LIST
- **Graph stats:** $NODES nodes, $EDGES edges, $COMMUNITIES communities
- **Health issues:** $ISSUE_COUNT
```

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

- Wiki pages refreshed with latest graph data.
- Orphaned graph-wiki pages removed.
- Session log updated with organize summary.

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

Wiki pages refreshed with latest graph data, orphaned pages removed, and summary reported to user.
