const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');
const os = require('os');
const net = require('net');

/**
 * Get the path to Moonton Claude Desktop settings file
 * All platforms: ~/.MoontonClaudeDesktop/config/settings.json
 */
function getSettingsPath() {
  return path.join(os.homedir(), '.MoontonClaudeDesktop', 'config', 'settings.json');
}

/**
 * Load settings from Moonton Claude Desktop configuration file
 */
function loadSettings() {
  const settingsPath = getSettingsPath();

  if (!fs.existsSync(settingsPath)) {
    console.log('[mcd] Settings file not found:', settingsPath);
    console.log('[mcd] Please configure environment variables in Moonton Claude Desktop');
    return {};
  }

  try {
    const content = fs.readFileSync(settingsPath, 'utf-8');
    return JSON.parse(content);
  } catch (e) {
    console.error('[mcd] Failed to read settings:', e.message);
    return {};
  }
}

/**
 * Get the path to CCR (Claude Code Router) directory
 * CCR provides LLM routing capabilities for using alternative models
 */
function getCcrPath() {
  // First try relative path (development/npm link scenario)
  // mcd-cli is at src-tauri/resources/mcd-cli, ccr is at src-tauri/resources/ccr
  const relativeCcrPath = path.join(__dirname, '..', '..', 'ccr');
  if (fs.existsSync(path.join(relativeCcrPath, 'cli.cjs'))) {
    return relativeCcrPath;
  }

  // Try from settings (if app stored it)
  const settings = loadSettings();
  if (settings.ccrPath && fs.existsSync(settings.ccrPath)) {
    return settings.ccrPath;
  }

  // Production fallback: app installation directory
  if (process.platform === 'win32') {
    const prodPath = path.join(process.env.LOCALAPPDATA || '', 'Programs', 'Moonton Claude Desktop', 'resources', 'ccr');
    if (fs.existsSync(path.join(prodPath, 'cli.cjs'))) return prodPath;

    // Alternative Windows install location
    const altPath = 'C:\\Program Files\\Moonton Claude Desktop\\resources\\ccr';
    if (fs.existsSync(path.join(altPath, 'cli.cjs'))) return altPath;
  } else if (process.platform === 'darwin') {
    const macPath = '/Applications/Moonton Claude Desktop.app/Contents/Resources/ccr';
    if (fs.existsSync(path.join(macPath, 'cli.cjs'))) return macPath;
  }

  return null;
}

/**
 * Check if CCR service is running on port 3456
 */
function checkCcrRunning() {
  return new Promise((resolve) => {
    const socket = new net.Socket();
    socket.setTimeout(1000);
    socket.on('connect', () => { socket.destroy(); resolve(true); });
    socket.on('error', () => resolve(false));
    socket.on('timeout', () => { socket.destroy(); resolve(false); });
    socket.connect(3456, '127.0.0.1');
  });
}

/**
 * Find Node.js executable path
 */
function getNodePath() {
  // Try MoontonClaudeDesktop's bundled Node.js
  if (process.platform === 'win32') {
    const bundledNode = path.join(os.homedir(), '.MoontonClaudeDesktop', 'nodejs', 'node.exe');
    if (fs.existsSync(bundledNode)) return bundledNode;
  }

  // Try relative bundled Node.js
  const relativeBundledNode = process.platform === 'win32'
    ? path.join(__dirname, '..', '..', 'nodejs-windows', 'node.exe')
    : path.join(__dirname, '..', '..', 'nodejs-macos', 'bin', 'node');
  if (fs.existsSync(relativeBundledNode)) return relativeBundledNode;

  // Fall back to system node
  return 'node';
}

/**
 * Ensure CCR service is running, start it if not
 */
async function ensureCcrRunning() {
  if (await checkCcrRunning()) {
    console.log('[mcd] CCR service already running on port 3456');
    return true;
  }

  const ccrPath = getCcrPath();
  if (!ccrPath) {
    console.log('[mcd] CCR not found, LLM router unavailable');
    return false;
  }

  console.log('[mcd] Starting CCR service...');
  const cliPath = path.join(ccrPath, 'cli.cjs');
  const nodePath = getNodePath();

  const ccr = spawn(nodePath, [cliPath, 'start'], {
    cwd: ccrPath,
    detached: true,
    stdio: 'ignore',
    windowsHide: true
  });
  ccr.unref();

  // Wait for service (max 5 seconds)
  for (let i = 0; i < 10; i++) {
    await new Promise(r => setTimeout(r, 500));
    if (await checkCcrRunning()) {
      console.log('[mcd] CCR service started successfully');
      return true;
    }
  }

  console.log('[mcd] Warning: CCR service failed to start');
  return false;
}

/**
 * Mask sensitive values in environment variables
 */
function maskValue(key, value) {
  const sensitivePatterns = ['KEY', 'TOKEN', 'SECRET', 'PASSWORD', 'CREDENTIAL'];
  const isSensitive = sensitivePatterns.some(pattern => key.toUpperCase().includes(pattern));

  if (isSensitive && value) {
    // Show first 4 chars and mask the rest
    if (value.length > 8) {
      return value.substring(0, 4) + '***' + value.substring(value.length - 4);
    }
    return '***';
  }
  return value;
}

/**
 * Run claude command with injected environment variables
 */
async function runClaude(args) {
  const settings = loadSettings();
  const envVars = settings.environmentVariables || {};
  const llmRouterConfig = settings.llmRouterConfig;

  // Display injected environment variables
  const envKeys = Object.keys(envVars);
  if (envKeys.length > 0) {
    console.log('[mcd] Injecting environment variables:');
    envKeys.forEach(key => {
      const maskedValue = maskValue(key, envVars[key]);
      console.log(`  ${key}=${maskedValue}`);
    });
    console.log('');
  } else {
    console.log('[mcd] No environment variables configured');
    console.log('[mcd] Configure them in Moonton Claude Desktop > Settings > Environment Variables');
    console.log('');
  }

  // Ensure CCR service is running for LLM routing
  const ccrRunning = await ensureCcrRunning();

  // Add CCR to PATH for ccr command access
  const ccrPath = getCcrPath();
  let pathEnv = process.env.PATH || '';
  if (ccrPath) {
    pathEnv = ccrPath + path.delimiter + pathEnv;
  }

  // Merge environment variables and spawn claude
  const childEnv = { ...process.env, ...envVars, PATH: pathEnv };

  // If CCR is running, override ANTHROPIC_BASE_URL to use local CCR router
  // This allows routing to alternative models like gpt-5.2
  if (ccrRunning) {
    childEnv.ANTHROPIC_BASE_URL = 'http://127.0.0.1:3456';
    console.log('[mcd] Using CCR router at http://127.0.0.1:3456');

    // Inject ANTHROPIC_MODEL from llmRouterConfig (separate from environment variables)
    // This allows LLM Router settings to be independent from general env var settings
    if (llmRouterConfig?.selectedProviderId && llmRouterConfig?.modelProviders) {
      const routerModel = llmRouterConfig.modelProviders[llmRouterConfig.selectedProviderId];
      if (routerModel) {
        childEnv.ANTHROPIC_MODEL = routerModel;
        console.log(`[mcd] Using LLM Router model: ${routerModel} (provider: ${llmRouterConfig.selectedProviderId})`);
      }
    }
  }

  // Determine claude command based on platform
  const claudeCmd = process.platform === 'win32' ? 'claude.cmd' : 'claude';

  const claude = spawn(claudeCmd, args, {
    env: childEnv,
    stdio: 'inherit',
    shell: process.platform === 'win32'
  });

  claude.on('error', (err) => {
    console.error('[mcd] Failed to start Claude:', err.message);
    console.log('[mcd] Please ensure Claude Code CLI is installed');
    console.log('[mcd] Install it in Moonton Claude Desktop > Settings > Claude Code');
    process.exit(1);
  });

  claude.on('exit', (code) => {
    process.exit(code || 0);
  });
}

module.exports = { runClaude, loadSettings, getSettingsPath };
