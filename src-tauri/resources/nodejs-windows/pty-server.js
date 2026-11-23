/**
 * Node.js PTY Server (Windows - ConPTY)
 *
 * This script provides a PTY service that communicates with Rust via stdin/stdout/stderr.
 *
 * Uses ConPTY (useConpty: true) — the native Windows pseudo-console API.
 * ConPTY provides proper terminal reflow, better Unicode support,
 * and is the recommended backend for Windows 10+.
 *
 * Protocol:
 * - Input (stdin): JSON messages, one per line
 *   - { "type": "spawn", "cols": 80, "rows": 24, "cwd": "/path", "env": {...} }
 *   - { "type": "write", "data": "..." }
 *   - { "type": "resize", "cols": 80, "rows": 24 }
 *   - { "type": "kill" }
 *
 * - Output (stdout): Raw PTY data (binary passthrough, zero serialization overhead)
 * - Control (stderr): JSON messages, one per line
 *   - { "type": "init", "platform": "win32", "version": "v24.x" }
 *   - { "type": "ready", "pid": 1234 }
 *   - { "type": "exit", "code": 0 }
 *   - { "type": "error", "message": "..." }
 */

const readline = require('readline');

let pty;
let ptyProcess = null;

// Control messages go to stderr (low frequency, JSON-encoded)
function sendControl(msg) {
  process.stderr.write(JSON.stringify(msg) + '\n');
}

// Try to load node-pty
try {
  pty = require('node-pty');
} catch (e) {
  sendControl({ type: 'error', message: `Failed to load node-pty: ${e.message}` });
  process.exit(1);
}

function getShell() {
  if (process.platform === 'win32') {
    // Use full path for PowerShell
    const systemRoot = process.env.SystemRoot || 'C:\\Windows';
    return `${systemRoot}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe`;
  }
  return process.env.SHELL || '/bin/bash';
}

/**
 * Build PowerShell prompt function based on configuration
 * @param {Object} config - Prompt configuration
 * @returns {string} PowerShell function definition
 */
function buildPromptFunction(config) {
  // If config not provided or disabled, use simple default prompt
  if (!config || !config.enabled) {
    return `function prompt { $p = $executionContext.SessionState.Path.CurrentLocation.Path; $e = [char]27; $b = [char]7; Write-Host -NoNewline "$e]7;file://localhost/$($p.Replace('\\\\','/'))$b"; "PS $p> " }`;
  }

  const parts = [];

  // OSC 7 directory tracking (always included for terminal integration)
  parts.push(`$p = $executionContext.SessionState.Path.CurrentLocation.Path`);
  parts.push(`$e = [char]27; $b = [char]7`);
  parts.push(`Write-Host -NoNewline "$e]7;file://localhost/$($p.Replace('\\\\','/'))$b"`);

  // Path display based on config
  if (config.showPath === 'basename') {
    parts.push(`$pathDisplay = Split-Path -Leaf $p`);
  } else if (config.showPath === 'short') {
    parts.push(`$pathDisplay = $p -replace [regex]::Escape($env:USERPROFILE), '~'`);
  } else {
    parts.push(`$pathDisplay = $p`);
  }

  // Git branch detection
  if (config.showGitBranch) {
    parts.push(`$gitBranch = ""`);
    parts.push(`$gitDirty = $false`);
    parts.push(`try { $gitBranch = git rev-parse --abbrev-ref HEAD 2>$null; if ($gitBranch) { $gitStatus = git status --porcelain 2>$null; $gitDirty = $gitStatus.Length -gt 0 } } catch {}`);
  }

  // Python virtual environment detection
  if (config.showPythonEnv) {
    parts.push(`$pyEnv = if ($env:VIRTUAL_ENV) { Split-Path -Leaf $env:VIRTUAL_ENV } else { "" }`);
  }

  // Get colors from config with defaults
  const colorPath = config.colors?.path || '39';
  const colorGitClean = config.colors?.gitClean || '34';
  const colorGitDirty = config.colors?.gitDirty || '33';
  const colorEnv = config.colors?.env || '35';

  let promptOutput = '';

  if (config.style === 'powerline') {
    // Powerline style with icons (using ASCII-safe alternatives for compatibility)
    // Use [char]0x276F for ❯ symbol (PowerShell 5.1 compatible)
    promptOutput = `
      $chevron = [char]0x276F;
      $prompt = "";
      ${config.showPythonEnv ? `if ($pyEnv) { $prompt += "$e[${colorEnv}m ($pyEnv) $e[0m" };` : ''}
      $prompt += "$e[${colorPath}m $pathDisplay $e[0m";
      ${config.showGitBranch ? `if ($gitBranch) { $gitColor = if ($gitDirty) { "${colorGitDirty}" } else { "${colorGitClean}" }; $gitIcon = if ($gitDirty) { "*" } else { "" }; $prompt += "$e[$($gitColor)m [$gitIcon$gitBranch] $e[0m" };` : ''}
      $prompt += " $chevron ";
      return $prompt
    `.replace(/\n\s+/g, ' ').trim();
  } else if (config.style === 'classic') {
    // Classic PowerShell style with Git info
    promptOutput = `
      $prompt = "PS ";
      $prompt += "$e[${colorPath}m$pathDisplay$e[0m";
      ${config.showGitBranch ? `if ($gitBranch) { $gitColor = if ($gitDirty) { "${colorGitDirty}" } else { "${colorGitClean}" }; $prompt += " $e[$($gitColor)m($gitBranch)$e[0m" };` : ''}
      ${config.showPythonEnv ? `if ($pyEnv) { $prompt += " $e[${colorEnv}m[$pyEnv]$e[0m" };` : ''}
      $prompt += "> ";
      return $prompt
    `.replace(/\n\s+/g, ' ').trim();
  } else {
    // Minimal style (default)
    // Use [char]0x276F for ❯ symbol (PowerShell 5.1 compatible)
    promptOutput = `
      $chevron = [char]0x276F;
      $prompt = "";
      ${config.showPythonEnv ? `if ($pyEnv) { $prompt += "$e[${colorEnv}m($pyEnv)$e[0m " };` : ''}
      $prompt += "$e[${colorPath}m$pathDisplay$e[0m";
      ${config.showGitBranch ? `if ($gitBranch) { $gitColor = if ($gitDirty) { "${colorGitDirty}" } else { "${colorGitClean}" }; $gitSymbol = if ($gitDirty) { "*" } else { "" }; $prompt += " $e[$($gitColor)m$gitBranch$gitSymbol$e[0m" };` : ''}
      $prompt += " $chevron ";
      return $prompt
    `.replace(/\n\s+/g, ' ').trim();
  }

  parts.push(promptOutput);

  return `function prompt { ${parts.join('; ')} }`;
}

function getShellArgs(initCommands, promptConfig) {
  if (process.platform === 'win32') {
    // Build PowerShell initialization commands
    const promptFunc = buildPromptFunction(promptConfig);
    // Import PSReadLine explicitly to ensure it's fully initialized, then remove Shift+Tab binding
    // This is needed because PSReadLine may set up its key bindings lazily after profile loads
    const unbindShiftTab = `Import-Module PSReadLine -ErrorAction SilentlyContinue; Remove-PSReadLineKeyHandler -Key Shift+Tab -ErrorAction SilentlyContinue`;

    // Clear CLAUDECODE inherited from parent process so `claude` works directly
    const clearClaudeCode = '$env:CLAUDECODE=$null';
    // Ensure UTF-8 encoding so braille spinner characters (U+2800-U+28FF) from
    // Claude Code's status line pass through ConPTY without corruption
    const setUtf8 = '[Console]::OutputEncoding = [System.Text.Encoding]::UTF8';
    let commands = [unbindShiftTab, clearClaudeCode, setUtf8, promptFunc];
    if (initCommands) {
      commands = commands.concat(initCommands);
    }

    const combinedCmd = commands.join('; ');
    return ['-NoLogo', '-NoExit', '-ExecutionPolicy', 'Bypass', '-Command', combinedCmd];
  }
  return ['-l']; // Login shell on macOS/Linux
}

function spawnPty(options) {
  if (ptyProcess) {
    sendControl({ type: 'error', message: 'PTY already spawned' });
    return;
  }

  const shell = getShell();

  // Build environment variable init commands for Windows
  let initCommands = null;
  if (process.platform === 'win32' && options.env) {
    initCommands = Object.entries(options.env).map(([k, v]) => {
      const escaped = v.replace(/"/g, '`"');
      return `$env:${k}="${escaped}"`;
    });
  }

  // Build Claude alias function definitions for PowerShell
  if (process.platform === 'win32' && options.aliases && Array.isArray(options.aliases)) {
    if (!initCommands) initCommands = [];
    const ALIAS_NAME_RE = /^[a-zA-Z][a-zA-Z0-9_-]*$/;
    const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
    // Escape for PowerShell double-quoted strings: backtick, double-quote, dollar
    const escapePS = (s) => s.replace(/`/g, '``').replace(/"/g, '`"').replace(/\$/g, '`$');
    for (const alias of options.aliases) {
      if (!alias.name || !ALIAS_NAME_RE.test(alias.name)) continue;
      // Always inject the three Anthropic vars (even if empty) to prevent
      // inheriting stale system environment variables.
      const envSetters = [
        `$env:ANTHROPIC_MODEL="${escapePS(alias.anthropic_model || '')}"`,
        `$env:ANTHROPIC_AUTH_TOKEN="${escapePS(alias.anthropic_auth_token || '')}"`,
        `$env:ANTHROPIC_BASE_URL="${escapePS(alias.anthropic_base_url || '')}"`,
      ];
      if (alias.custom_env_vars && typeof alias.custom_env_vars === 'object') {
        for (const [k, v] of Object.entries(alias.custom_env_vars)) {
          if (!ENV_KEY_RE.test(k)) continue;
          if (v) {
            envSetters.push(`$env:${k}="${escapePS(String(v))}"`);
          }
        }
      }
      // Clear CLAUDECODE before AND after claude to prevent "nested session" detection.
      // Claude sets CLAUDECODE during execution; must clear after exit too.
      const clearNested = '$env:CLAUDECODE=$null';
      const funcBody = envSetters.length > 0
        ? `${clearNested}; ${envSetters.join('; ')}; claude @args; ${clearNested}`
        : `${clearNested}; claude @args; ${clearNested}`;
      initCommands.push(`function ${alias.name} { ${funcBody} }`);
    }
  }

  // Get prompt configuration from spawn options
  const promptConfig = options.promptConfig || null;
  const args = getShellArgs(initCommands, promptConfig);

  const ptyOptions = {
    name: 'xterm-256color',
    cols: options.cols || 80,
    rows: options.rows || 24,
    cwd: options.cwd || process.cwd(),
    env: { ...process.env, ...options.env },
    useConpty: true,  // Use ConPTY — native Windows pseudo-console API
  };

  try {
    ptyProcess = pty.spawn(shell, args, ptyOptions);

    // PTY data goes directly to stdout (zero serialization overhead)
    ptyProcess.onData((data) => {
      process.stdout.write(data);
    });

    ptyProcess.onExit(({ exitCode, signal }) => {
      sendControl({ type: 'exit', code: exitCode, signal });
      ptyProcess = null;
    });

    sendControl({ type: 'ready', pid: ptyProcess.pid });
  } catch (e) {
    sendControl({ type: 'error', message: `Failed to spawn PTY: ${e.message}` });
  }
}

function writePty(data) {
  if (!ptyProcess) {
    sendControl({ type: 'error', message: 'PTY not spawned' });
    return;
  }

  try {
    // Data is already UTF-8 text, write directly (no base64 decoding needed)
    ptyProcess.write(data);
  } catch (e) {
    sendControl({ type: 'error', message: `Write error: ${e.message}` });
  }
}

function resizePty(cols, rows) {
  if (!ptyProcess) {
    sendControl({ type: 'error', message: 'PTY not spawned' });
    return;
  }

  try {
    ptyProcess.resize(cols, rows);
  } catch (e) {
    sendControl({ type: 'error', message: `Resize error: ${e.message}` });
  }
}

function killPty() {
  if (!ptyProcess) {
    return;
  }

  try {
    ptyProcess.kill();
    ptyProcess = null;
  } catch (e) {
    sendControl({ type: 'error', message: `Kill error: ${e.message}` });
  }
}

// Read commands from stdin
const rl = readline.createInterface({
  input: process.stdin,
  output: null,
  terminal: false,
});

rl.on('line', (line) => {
  try {
    const msg = JSON.parse(line);

    switch (msg.type) {
      case 'spawn':
        spawnPty(msg);
        break;
      case 'write':
        writePty(msg.data);
        break;
      case 'resize':
        resizePty(msg.cols, msg.rows);
        break;
      case 'kill':
        killPty();
        break;
      default:
        sendControl({ type: 'error', message: `Unknown command: ${msg.type}` });
    }
  } catch (e) {
    sendControl({ type: 'error', message: `Parse error: ${e.message}` });
  }
});

rl.on('close', () => {
  killPty();
  process.exit(0);
});

// Signal that we're ready to receive commands
sendControl({ type: 'init', platform: process.platform, version: process.version });
