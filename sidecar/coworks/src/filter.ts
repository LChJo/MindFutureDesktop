/**
 * One-shot filter script for bot_filter_message.
 *
 * Protocol:
 *   stdin:  { "content": string, "systemPrompt": string, "model": string }
 *   stdout: { "pass": boolean, "answer": string }
 *
 * Reads exactly one JSON line, runs a single-turn Agent SDK query in chat mode
 * (no tools, no session persistence), writes the pass/fail result, then exits.
 * Fails open on any error or ambiguous result so messages are never silently lost.
 */
import { query } from '@anthropic-ai/claude-agent-sdk'
import * as readline from 'readline'

const MODEL_MAP: Record<string, string> = {
  Haiku: 'claude-haiku-4-5-20251001',
  Sonnet: 'claude-sonnet-4-6',
  Opus: 'claude-opus-4-6',
}

function writeResult(pass: boolean, answer = ''): void {
  process.stdout.write(JSON.stringify({ pass, answer }) + '\n')
}

async function main(): Promise<void> {
  const rl = readline.createInterface({ input: process.stdin, terminal: false })

  for await (const line of rl) {
    if (!line.trim()) continue

    const { content, systemPrompt, model } = JSON.parse(line) as {
      content: string
      systemPrompt: string
      model: string
    }

    try {
      const filterPrompt = `根据以下规则判断消息是否应该通过，只回复 YES（通过）或 NO（拦截），不要有任何其他内容。\n\n规则：${systemPrompt}\n\n消息：${content}`
      const response = query({
        prompt: filterPrompt,
        options: {
          cwd: process.cwd(),
          model: MODEL_MAP[model] ?? model,
          maxTurns: 1,
          // tools: [] disables all built-in tools from the model's context (chat mode).
          // allowedTools only controls auto-approval and cannot restrict available tools.
          tools: [],
          permissionMode: 'dontAsk',
          // Do not write session history to ~/.claude/projects/ for ephemeral filter calls.
          persistSession: false,
        },
      })

      let gotResult = false
      for await (const msg of response) {
        // Fail-open immediately on assistant-level errors (auth, billing, rate-limit, etc.)
        if (msg.type === 'assistant' && msg.error) {
          writeResult(true, `[assistant-error:${msg.error}]`)
          process.exit(0)
        }

        if (msg.type === 'result') {
          gotResult = true
          // Error result subtypes (error_during_execution, error_max_turns, etc.) → fail-open.
          if (msg.subtype !== 'success' || msg.is_error) {
            writeResult(true, `[result-error:${msg.subtype}]`)
            process.exit(0)
          }
          // Use result.result as the authoritative final answer from the model.
          // Empty answer → fail-open rather than silently blocking.
          const trimmed = msg.result.trim()
          writeResult(trimmed.length === 0 || trimmed.toUpperCase().startsWith('YES'), trimmed.slice(0, 200))
          process.exit(0)
        }
      }

      // Stream ended without a result message → fail-open.
      if (!gotResult) {
        writeResult(true, '[no-result]')
        process.exit(0)
      }
    } catch {
      writeResult(true, '[catch:fail-open]') // fail-open
    }

    process.exit(0)
  }
}

main().catch(() => {
  writeResult(true)
  process.exit(0)
})
