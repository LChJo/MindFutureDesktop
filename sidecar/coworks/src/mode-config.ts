/**
 * Mode-specific configurations for Coworks SDK
 * Defines different parameters for Chat, Code, and Cowork modes
 */

import type { Options } from '@anthropic-ai/claude-agent-sdk'
import type { CoworkMode, PluginInfo } from './types.js'

// 通用语言设置
const LANGUAGE_INSTRUCTION = `

【重要】请始终使用中文回复用户，包括所有解释、说明和输出内容。`

type SystemPromptConfig = NonNullable<Options['systemPrompt']>

// System prompts for each mode
const SYSTEM_PROMPTS: Record<CoworkMode, SystemPromptConfig> = {
  chat: `你是一个友好的 AI 助手。请用自然、亲切的语气与用户交流。
你不需要执行任何代码或文件操作，只需要进行纯对话交流。
回答用户的问题时，请简洁明了，必要时可以使用 markdown 格式来组织内容。${LANGUAGE_INSTRUCTION}`,

  code: {
    type: 'preset',
    preset: 'claude_code',
    append: `
【身份设定】
你是一个专业的全栈开发助手，帮助用户进行项目开发。
你可以：
- 阅读、编写和编辑代码文件
- 执行终端命令
- 搜索和浏览项目文件
- 提供代码审查和优化建议
- 使用 Agent/Task 工具调用专项子代理完成复杂任务

请根据项目上下文提供准确、高效的开发建议和代码实现。${LANGUAGE_INSTRUCTION}`,
  },

  cowork: `你是一个文件管理助手，帮助用户完成特定的文件操作任务。
你可以：
- 读取和修改文件内容
- 在指定目录中搜索文件
- 执行必要的文件操作命令

请专注于用户指定的任务，在任务工作目录范围内高效完成文件操作。${LANGUAGE_INSTRUCTION}`,

  claudeapps: {
    type: 'preset',
    preset: 'claude_code',
    append: `
【身份设定】
你是一个 Claude Apps 开发助手，帮助用户创建和维护动态功能应用。

默认创建纯前端应用，不要生成 backend 目录。

每个应用是一个目录，包含以下文件结构：
- manifest.json: 应用元数据（name, displayName, version, description, icon, window, capabilities, backend）
- frontend/index.html: 前端入口页面
- frontend/app.js: 前端逻辑
- frontend/style.css: 样式

manifest.json 模板：
{
  "name": "my-app",
  "displayName": "我的应用",
  "version": "1.0.0",
  "description": "应用描述",
  "icon": "fa-solid fa-cube",
  "window": { "width": 800, "height": 600, "minWidth": 400, "minHeight": 300 },
  "capabilities": { "network": false, "localStorage": false, "exec": false },
  "backend": { "enabled": false }
}

iframe 内可通过 window.mcd 对象调用宿主 API：
- mcd.invoke('mcd:readFile', {path}) - 读取函数目录内文件
- mcd.invoke('mcd:writeFile', {path, content}) - 写入函数目录内文件
- mcd.setTitle(title) - 设置窗口标题
- mcd.notify(message, level) - 显示通知（level: 'info' | 'success' | 'warning' | 'error'）
- mcd.openExternal(url) - 在系统默认浏览器中打开URL（仅支持 http/https）
- mcd.exec(command, args, options) - 执行系统命令（需要在 manifest 中声明 "exec": true）
  - command: 可执行程序名或路径（如 'python', 'node', 'git'）
  - args: 参数数组（如 ['script.py', '--flag']）
  - options: { cwd?: string, env?: Record<string,string>, background?: boolean, timeout_ms?: number }
  - 同步模式（默认）: 等待命令执行完毕，返回 { stdout, stderr, exitCode }
  - 后台模式（background: true）: 立即返回 { pid }，进程在后台继续运行
- mcd.kill(pid) - 终止通过 mcd.exec 启动的后台进程（需要 exec capability）

注意iframe和普通前端的区别：
- 只能访问函数目录内的文件，无法访问其他目录
- 只能使用 mcd.invoke 调用宿主 API，无法直接使用 Node.js API
- 需要通过 iframe 验证接口安全性和权限
- exec capability 允许应用执行系统命令，仅在用户明确需要时启用

你可以创建新应用或修改已有应用的代码。请根据用户的描述生成完整、可运行的应用代码。${LANGUAGE_INSTRUCTION}`,
  },
}

// Allowed tools for each mode
// Note: SDK doesn't have 'LS' tool, use Glob or Bash(ls) instead
const ALLOWED_TOOLS: Record<CoworkMode, string[]> = {
  chat: ['Read'], // Read-only for viewing attachments
  code: [
    'Read',
    'Write',
    'Edit',
    'Bash',
    'Glob',
    'Grep',
    'Task',
    'WebFetch',
    'WebSearch',
    'NotebookEdit',
    'TodoWrite',
    'AskUserQuestion',
  ],
  cowork: ['Read', 'Write', 'Edit', 'Bash', 'Glob', 'Grep'],
  claudeapps: ['Read', 'Write', 'Edit', 'Bash', 'Glob', 'Grep', 'Task', 'WebFetch', 'WebSearch'],
}

// Max turns for each mode
const MAX_TURNS: Record<CoworkMode, number> = {
  chat: 10, // Allow multiple turns for reading attachments
  code: 50, // Multi-turn for code mode
  cowork: 50, // Multi-turn for cowork mode
  claudeapps: 50, // Multi-turn for Claude Apps mode
}

export interface ModeConfig {
  systemPrompt: SystemPromptConfig
  allowedTools: string[]
  maxTurns: number
}

/**
 * Get the configuration for a specific mode
 */
export function getModeConfig(mode: CoworkMode): ModeConfig {
  return {
    systemPrompt: SYSTEM_PROMPTS[mode],
    allowedTools: ALLOWED_TOOLS[mode],
    maxTurns: MAX_TURNS[mode],
  }
}

/**
 * Build SDK options based on mode, workDir, and model
 *
 * SDK 参数说明 (claude-agent-sdk)：
 * - cwd: 工作目录路径
 * - systemPrompt: 系统提示词（定义 AI 角色）
 *   - string: 完全替换默认提示
 *   - { type: 'preset', preset: 'claude_code', append? }: 使用 Claude Code 默认提示并可追加
 * - model: 模型 ID
 * - maxTurns: 最大对话轮次（工具调用循环次数）
 * - allowedTools: 允许使用的工具列表
 * - permissionMode: 权限模式
 * - plugins: 插件列表（本地插件路径）
 */
export function buildOptions(
  mode: CoworkMode,
  workDir: string,
  model: string,
  userSystemPrompt?: string,
  plugin?: PluginInfo,
  usePreset?: boolean
): Options {
  const config = getModeConfig(mode)

  // 如果有 plugin，需要添加 Skill 工具到 allowedTools
  const allowedTools = plugin?.skillName
    ? [...config.allowedTools, 'Skill']
    : config.allowedTools

  // 系统提示词配置
  // 优先级：
  //   1. Plugin/Skill 模式：preset + skill 指令追加
  //   2. usePreset=true + 用户自定义：preset + 用户提示作为 append（Bot 场景）
  //   3. 用户自定义（完全替换，保留原有行为）
  //   4. 默认模式配置（code/claudeapps 为 preset 对象，chat/cowork 为字符串）
  let systemPrompt: Options['systemPrompt']

  if (plugin?.skillName) {
    // 有 skill 时，使用 preset 模式并追加 skill 指令
    // 注意：SDK 使用插件目录名作为插件名，路径结构为 .../plugin-name/version
    // 所以需要从路径中提取版本号作为前缀
    const pluginVersion = plugin.path.split(/[/\\]/).pop() || ''
    const fullSkillName = `${pluginVersion}:${plugin.skillName}`

    const skillInstruction = `

【身份设定】
你是一个专注的 Worker，负责执行用户指定的任务。你已被分配了一个插件技能来完成工作。
- 你的核心职责是使用已加载的插件技能高效完成用户的请求
- 当用户的请求与你的技能相关时，立即调用插件，不要犹豫
- 保持专注，不要偏离任务目标

【当前技能】
技能名称: ${plugin.skillName}
技能描述: ${plugin.skillDescription}

【调用方式】
使用 Skill 工具调用 /${fullSkillName} 来执行任务。
${LANGUAGE_INSTRUCTION}`
    systemPrompt = {
      type: 'preset' as const,
      preset: 'claude_code' as const,
      append: skillInstruction,
    }
  } else if (usePreset && userSystemPrompt) {
    // usePreset=true：将用户自定义提示作为 append 追加到 claude_code preset（Bot 场景）
    systemPrompt = {
      type: 'preset' as const,
      preset: 'claude_code' as const,
      append: userSystemPrompt,
    }
  } else if (usePreset) {
    // usePreset=true 但无自定义提示：使用默认模式配置（code 模式本身已是 preset）
    systemPrompt = config.systemPrompt
  } else if (userSystemPrompt) {
    // 用户自定义：完全替换（保留原有行为）
    systemPrompt = userSystemPrompt
  } else {
    // 默认模式提示词（code/claudeapps 为 preset 对象，chat/cowork 为字符串）
    systemPrompt = config.systemPrompt
  }

  const options: Options = {
    // 工作目录
    cwd: workDir || process.cwd(),
    // Node.js 可执行文件完整路径（避免 SDK 用裸 "node" 命令导致 ENOENT）
    executable: process.execPath as Options['executable'],
    // 系统提示词
    systemPrompt,
    // 最大轮次
    maxTurns: config.maxTurns,
    // 允许的工具列表（如果有 skill 则包含 Skill 工具）
    allowedTools,
    // 权限模式：绕过权限检查
    permissionMode: 'bypassPermissions',
    // 加载 cwd 中的 CLAUDE.md 文件
    settingSources: ['project'],
    // 模型（auto 时优先使用 ANTHROPIC_MODEL 环境变量，否则使用 claudecode_common）
    ...(model ? { model: model === 'auto' ? (process.env.ANTHROPIC_MODEL || 'Sonnet') : model } : {}),
    // 插件（如果有 skill）
    ...(plugin?.path
      ? { plugins: [{ type: 'local' as const, path: plugin.path }] }
      : {}),
  }

  return options
}
