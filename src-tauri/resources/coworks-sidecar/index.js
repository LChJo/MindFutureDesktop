/**
 * Coworks Sidecar - JSON Lines Protocol
 *
 * Communication happens via stdin/stdout using JSON Lines format.
 * Each line is a complete JSON object.
 *
 * Input messages:
 * - { type: 'start', sessionId, workDir, model, prompt, mode, systemPrompt?, usePreset?, plugin?, attachments?, sdkSessionId? }
 * - { type: 'cancel', sessionId }
 * - { type: 'ping' }
 *
 * Output messages:
 * - { type: 'ready' }
 * - { type: 'pong' }
 * - { type: 'text_delta', sessionId, content }
 * - { type: 'tool_use', sessionId, toolId, toolName, input }
 * - { type: 'tool_result', sessionId, toolId, summary }
 * - { type: 'done', sessionId }
 * - { type: 'error', sessionId, message }
 * - { type: 'cancelled', sessionId }
 */
import * as readline from 'readline';
import { startSession, cancelSession } from './claude-service.js';
// 日志前缀
const LOG_PREFIX = '[sidecar]';
// 日志工具函数
function log(level, ...args) {
    const timestamp = new Date().toISOString();
    console.error(`${timestamp} ${LOG_PREFIX} [${level}]`, ...args);
}
function emit(message) {
    const json = JSON.stringify(message);
    // Use process.stdout.write to avoid buffering issues when piped
    process.stdout.write(json + '\n');
    // 完整日志输出
    log('INFO', `[Sidecar OUT] ${json}`);
}
async function handleMessage(line, message) {
    // 完整输入日志
    log('INFO', `[Sidecar IN] ${line}`);
    switch (message.type) {
        case 'ping':
            emit({ type: 'pong' });
            break;
        case 'start':
            // Start session in background (don't await)
            startSession(message.sessionId, message.workDir, message.model, message.prompt, message.mode, message.systemPrompt, message.plugin, message.attachments, message.sdkSessionId, message.usePreset).catch((error) => {
                log('ERROR', 'Session error:', error);
                emit({
                    type: 'error',
                    sessionId: message.sessionId,
                    message: error instanceof Error ? error.message : String(error),
                });
            });
            break;
        case 'cancel':
            log('INFO', `Cancelling session: ${message.sessionId}`);
            cancelSession(message.sessionId);
            break;
        default:
            log('WARN', `Unknown message type: ${message.type}`);
            break;
    }
}
async function main() {
    log('INFO', 'Sidecar 启动中...');
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
        terminal: false,
    });
    // Signal ready
    emit({ type: 'ready' });
    log('INFO', 'Sidecar 已就绪，等待输入...');
    for await (const line of rl) {
        if (!line.trim())
            continue;
        try {
            const message = JSON.parse(line);
            await handleMessage(line, message);
        }
        catch (error) {
            log('ERROR', 'Failed to parse message:', error, 'Raw line:', line);
        }
    }
    log('INFO', 'Sidecar 关闭');
}
main().catch((error) => {
    log('ERROR', 'Fatal error:', error);
    process.exit(1);
});
//# sourceMappingURL=index.js.map