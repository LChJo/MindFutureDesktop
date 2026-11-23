# /debug

> **Invoked by:** user-session (for analyzing and fixing bugs outside the /verify → /fix pipeline, with optional `--dispatch`)

## 会话前导

执行这个 command 之前，先完成下面的初始化步骤：

1. 阅读 `.claude/progress/session-log.md`，先理解当前状态。
2. 从 `.claude/milestones/` 确认当前活动的 milestone 和 task。
3. 如果 milestone 或 wiki 自上次会话后发生变化，先运行 `/sync-progress`。
4. 读取当前 task 上下文所适用的 `.claude/rules/` 文件。

## Objective

分析并快速修复 bug，走"复现 → 诊断根因 → 修复 → 回归验证"流程，保留 harness 核心保证（session-log 记录、原子提交），但跳过完整的 milestone/sprint 流程。

使用 `--full` flag 可启用完整回归验证并强制添加回归测试用例。

使用 `--dispatch` flag 可将 bug 修复分派给拥有该模块上下文的 `core-{module}` agent 执行。`--full` 和 `--dispatch` 可组合使用。

## Workflow Policy

- `completeness_mode`: `balanced`
- `ownership_mode`: `solo`
- `question_protocol`: `structured`
- `review_depth`: `standard`
- `require_ascii_diagrams`: `auto`
- `design_review_mode`: `none`
- `extraction_mode`: `standard`

## Steps

### Step 1 — 解析参数

解析 `$ARGUMENTS`：
- `--full` flag → 存储为 `$FULL_MODE`（true/false），启用完整回归验证 + 强制回归测试
- `--dispatch` flag → 存储为 `$DISPATCH_MODE`（true/false），启用模块 Agent 分派
- 剩余文本 → bug 描述 `$BUG_DESCRIPTION`

如果 `$BUG_DESCRIPTION` 为空，使用 AskUserQuestion 询问：

```
AskUserQuestion(
  header: "Debug",
  question: "请描述你遇到的 bug（症状、错误信息、复现步骤等）",
  options: []
)
```

如果仍然为空，提示用户必须提供 bug 描述。

展示模式 banner：

**`--full --dispatch` 模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► DEBUG (FULL + DISPATCH)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

◆ 完整回归验证 + 回归测试 + 模块 Agent 分派已启用
```

**`--full` 模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► DEBUG (FULL)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

◆ 完整回归验证 + 回归测试已启用
```

**`--dispatch` 模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► DEBUG (DISPATCH)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

◆ 模块 Agent 分派已启用
```

**默认模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► DEBUG
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

◆ $BUG_DESCRIPTION
```

### Step 2 — 读取上下文 & 收集线索

1. 读 `session-state.json` 了解当前 milestone 和任务状态。
2. 读 `session-log.md` 了解近期进展和决策，检查是否有相关的已知问题。
3. 读与 bug 相关的规则文件和领域文档。
4. 收集 bug 线索：
   - 错误信息、堆栈跟踪、日志输出
   - 复现步骤或触发条件
   - 受影响的模块和文件
   - 最近的相关变更（git log）

### Step 2.5 — Dispatch Assessment（仅 `--dispatch` 模式）

**如果 `$DISPATCH_MODE` 为 false，直接跳到 Step 3。**

基于 Step 2 收集的线索，评估是否可以分派给模块 Agent：

1. 读 `.claude/wiki/modules/` 构建 module-slug → paths 映射表。
2. 将受影响的文件路径匹配到模块所有权。
3. 如果匹配到单个 `core-{module}` agent：

   展示 Dispatch Proposal：

   ```
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    DISPATCH PROPOSAL
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   Module: $MODULE_NAME
   Agent: core-$MODULE_SLUG
   Bug: $BUG_DESCRIPTION
   Affected files: $FILE_LIST
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   ```

   使用 AskUserQuestion 确认：
   - **"确认 dispatch"** → 构建 sprint-context，dispatch 给 `core-{module}` agent 执行完整 debug 流程（复现 → 诊断 → 修复 → 验证）。等待结果后展示给用户 → **EXIT**。
   - **"继续本地处理"** → 跳过 dispatch，继续 Step 3。

4. 如果匹配多个模块 → 显示模块列表，让用户选择目标模块或本地处理。
5. 如果无匹配 → 继续 Step 3。

### Step 3 — 复现 & 诊断

**3a. 复现**

尝试复现 bug：
- 运行相关测试，确认失败
- 或执行触发路径，确认错误发生
- 如果无法复现，向用户说明并请求更多信息

**3b. 诊断根因**

追踪调用链定位根因，分类问题类型：
- **代码逻辑错误** — 条件判断、算法、数据处理问题
- **契约违反** — 模块间接口不匹配、类型错误
- **边界条件遗漏** — 空值、溢出、并发竞争
- **外部依赖问题** — 第三方库、配置、环境差异

**3c. 展示诊断结果**

向用户展示：

1. **根因** — 问题的本质原因，指向具体代码位置。
2. **影响范围** — 哪些功能/模块受影响。
3. **修复方案** — 最小正确修复的具体步骤。
4. **风险评估** — 修复是否可能引入新问题。

**等待用户确认后再继续。** 如果用户对诊断有异议或补充信息，修正后重新展示。

### Step 4 — 实施修复

按确认的方案执行：

1. 应用最小正确修复，不做超出修复范围的重构。
2. 遵守模块所有权边界。
3. 保持契约明确，修复不应改变公共接口。

**`--full` 模式额外要求：**
- 必须添加回归测试，覆盖触发该 bug 的具体场景。
- 回归测试应在修复前失败（RED），修复后通过（GREEN）。

### Step 5 — 回归验证

**默认模式：**
1. 运行与 bug 相关的测试，确认修复有效。
2. 运行 bug 所在模块的测试套件，确认无新回归。
3. 记录验证结果为 `$VERIFICATION_RESULT`。

**`--full` 模式：**
1. 运行 runbook 中的完整 build、test、quality 命令（参见 verification_commands）。
2. 确认回归测试通过。
3. 确认无其他测试被破坏。
4. 记录验证结果为 `$VERIFICATION_RESULT`（passed / failed / partial）。

如验证失败，执行最小修复后重新验证（最多 2 轮）。

### Step 6 — 记录与提交

**6a. 更新 session-log（调用 skill，不要直接写）**

调用 `@write-session-log` 传入：

- `entry_type`: `"debug"`
- `summary`: $BUG_DESCRIPTION
- `evidence`:
  - date: $DATE
  - mode: $MODE (default | full)
  - root_cause: $ROOT_CAUSE（一句话概括根因）
  - fix: $FIX_SUMMARY（修复措施概要）
  - changes: 变更文件列表
  - regression_test: $REGRESSION_TEST（--full 模式）或 "not added"（默认模式）
  - verification: $VERIFICATION_RESULT
  - commit: $COMMIT_HASH
- `pending_add`: 如果修复未完全解决，或留下观察项/后续任务，列出；否则 `[]`
- `pending_resolve`: 如果本次解决了 Pending 里原先的 bug 条目，列出其文本

skill 负责把条目写入 Recent Activity 段，更新 Pending 层，并复核 150 行硬限。

**6b. 原子提交**

将所有代码变更（修复 + 回归测试）和 session-log 更新作为一次原子提交：

```
fix(debug): $BUG_DESCRIPTION
```

**6c. 展示完成摘要**

**`--full` 模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► BUG FIX COMPLETE (FULL)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Bug: $BUG_DESCRIPTION
Root Cause: $ROOT_CAUSE
Files: $CHANGED_FILES
Regression Test: $REGRESSION_TEST
Verification: $VERIFICATION_RESULT
Commit: $COMMIT_HASH
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**默认模式：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 HARNESS ► BUG FIX COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Bug: $BUG_DESCRIPTION
Root Cause: $ROOT_CAUSE
Files: $CHANGED_FILES
Verification: $VERIFICATION_RESULT
Commit: $COMMIT_HASH
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Decision Protocol

See `.claude/rules/decision-protocol.md` (auto-loaded via settings.json).

## Required Artifacts

- 修复代码已提交
- `session-log.md` 包含 bug fix 记录条目（含根因分析）
- 回归验证已通过
- （`--full` 模式）回归测试已添加

## Completion Status

See `.claude/rules/completion-status.md` (auto-loaded via settings.json).

## Exit Condition

Bug 修复已提交，回归验证通过，session-log 已更新 bug fix 记录（含根因），用户看到完成摘要。
