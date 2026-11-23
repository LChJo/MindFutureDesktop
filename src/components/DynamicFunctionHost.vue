<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { homeDir, join } from '@tauri-apps/api/path'
import { useDynamicFunctionStore } from '@/stores/dynamicFunctionStore'

interface DynamicFunctionManifest {
  name: string
  displayName: string
  version: string
  description: string
  icon: string
  window: { width: number; height: number; minWidth: number; minHeight: number }
  capabilities: { network: boolean; localStorage: boolean; exec: boolean }
  backend: { enabled: boolean; dependencies: Record<string, string> }
}

const props = defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()

const dfStore = useDynamicFunctionStore()

const iframeRef = ref<HTMLIFrameElement | null>(null)
const loading = ref(true)
const error = ref('')
const backendPort = ref<number | null>(null)
const manifest = ref<DynamicFunctionManifest | null>(null)

const functionName = computed(() =>
  (props.windowData?.functionName as string) || ''
)

/** Build the bridge script that provides mcd.* API inside the iframe */
function buildBridgeScript(port: number | null): string {
  return `
<script>
(function() {
  var mcd = {};
  var _listeners = {};
  var _reqId = 0;
  var _pending = {};

  mcd.backend = function(path, options) {
    ${port ? `
    var url = 'http://127.0.0.1:${port}' + (path.startsWith('/') ? path : '/' + path);
    return fetch(url, options || {}).then(function(r) { return r.json(); });
    ` : `
    return Promise.reject(new Error('No backend available'));
    `}
  };

  mcd.invoke = function(command, args) {
    return new Promise(function(resolve, reject) {
      var id = ++_reqId;
      _pending[id] = { resolve: resolve, reject: reject };
      window.parent.postMessage({
        type: 'mcd:invoke',
        id: id,
        command: command,
        args: args || {}
      }, '*');
    });
  };

  mcd.setTitle = function(title) {
    window.parent.postMessage({ type: 'mcd:setTitle', title: title }, '*');
  };

  mcd.notify = function(message, level) {
    window.parent.postMessage({ type: 'mcd:notify', message: message, level: level || 'info' }, '*');
  };

  mcd.on = function(event, callback) {
    if (!_listeners[event]) _listeners[event] = [];
    _listeners[event].push(callback);
  };

  mcd.off = function(event, callback) {
    if (!_listeners[event]) return;
    _listeners[event] = _listeners[event].filter(function(cb) { return cb !== callback; });
  };

  mcd.openExternal = function(url) {
    return mcd.invoke('mcd:openExternal', { url: url });
  };

  mcd.exec = function(command, args, options) {
    return mcd.invoke('mcd:exec', {
      command: command,
      args: args || [],
      options: options || {}
    });
  };

  mcd.kill = function(pid) {
    return mcd.invoke('mcd:kill', { pid: pid });
  };

  window.addEventListener('message', function(e) {
    var data = e.data;
    if (!data || !data.type) return;

    if (data.type === 'mcd:response' && _pending[data.id]) {
      if (data.error) {
        _pending[data.id].reject(new Error(data.error));
      } else {
        _pending[data.id].resolve(data.result);
      }
      delete _pending[data.id];
    }

    if (data.type === 'mcd:event' && _listeners[data.event]) {
      _listeners[data.event].forEach(function(cb) { cb(data.payload); });
    }
  });

  window.mcd = mcd;
})();
<\/script>
`
}

/** Read frontend files and build the iframe srcdoc */
async function loadFunction() {
  if (!functionName.value) {
    error.value = 'No function name provided'
    loading.value = false
    return
  }

  try {
    const home = await homeDir()
    const funcDir = await join(home, '.MoontonClaudeDesktop', 'dynamic-functions', functionName.value)

    // Read manifest
    const manifestContent = await readTextFile(await join(funcDir, 'manifest.json'))
    manifest.value = JSON.parse(manifestContent)

    // Start backend if enabled
    if (manifest.value?.backend?.enabled) {
      try {
        const port = await dfStore.startBackend(functionName.value)
        backendPort.value = port
      } catch (e) {
        console.error('Failed to start backend:', e)
      }
    }

    // Read frontend files
    const htmlPath = await join(funcDir, 'frontend', 'index.html')
    let html = await readTextFile(htmlPath)

    // Try to read and inline CSS
    try {
      const cssPath = await join(funcDir, 'frontend', 'style.css')
      const css = await readTextFile(cssPath)
      html = html.replace('</head>', `<style>${css}</style></head>`)
    } catch {
      // No style.css, that's fine
    }

    // Try to read and inline JS
    try {
      const jsPath = await join(funcDir, 'frontend', 'app.js')
      const js = await readTextFile(jsPath)
      html = html.replace('</body>', `<script>${js}<\/script></body>`)
    } catch {
      // No app.js, that's fine
    }

    // Inject bridge script at the top of <head>
    const bridge = buildBridgeScript(backendPort.value)
    if (html.includes('<head>')) {
      html = html.replace('<head>', `<head>${bridge}`)
    } else {
      html = bridge + html
    }

    // Set srcdoc on iframe
    if (iframeRef.value) {
      iframeRef.value.srcdoc = html
    }

    loading.value = false
  } catch (e) {
    error.value = `Failed to load function: ${e}`
    loading.value = false
  }
}

/** Handle postMessage from iframe */
function handleMessage(event: MessageEvent) {
  if (!iframeRef.value || event.source !== iframeRef.value.contentWindow) return

  // srcdoc iframes without allow-same-origin have origin "null".
  // With allow-same-origin (localStorage capability), the iframe inherits
  // the parent origin (e.g. "https://tauri.localhost").
  // We already verified event.source === our iframe's contentWindow above,
  // so accept both origins.
  if (event.origin !== 'null' && event.origin !== window.location.origin) {
    console.warn('Rejected message from unexpected origin:', event.origin)
    return
  }

  const data = event.data
  if (!data || !data.type) return

  switch (data.type) {
    case 'mcd:setTitle':
      // Could emit to parent window to update title
      break

    case 'mcd:notify':
      // Simple notification via console for now
      if (data.level === 'error') {
        console.error(`[${functionName.value}]`, data.message)
      } else {
        console.info(`[${functionName.value}]`, data.message)
      }
      break

    case 'mcd:invoke': {
      handleInvoke(data.id, data.command, data.args)
      break
    }
  }
}

/** Validate a relative file path for security */
function validateRelativePath(filePath: string): void {
  if (!filePath) throw new Error('File path cannot be empty')
  if (filePath.includes('..')) throw new Error('Path traversal not allowed')
  if (filePath.startsWith('/') || filePath.startsWith('\\')) throw new Error('Absolute paths not allowed')
  if (/^[A-Za-z]:/.test(filePath)) throw new Error('Absolute paths not allowed')
  if (filePath.includes('\0')) throw new Error('Null bytes not allowed in path')
}

/** Handle mcd.invoke() calls from iframe */
async function handleInvoke(id: number, command: string, args: Record<string, unknown>) {
  const iframe = iframeRef.value
  if (!iframe?.contentWindow) return

  // Whitelist of allowed commands
  const allowed = ['mcd:readFile', 'mcd:writeFile', 'mcd:openExternal', 'mcd:exec', 'mcd:kill']
  if (!allowed.includes(command)) {
    iframe.contentWindow.postMessage({
      type: 'mcd:response',
      id,
      error: `Command not allowed: ${command}`
    }, '*')
    return
  }

  try {
    let result: unknown

    if (command === 'mcd:readFile') {
      const filePath = args.path as string
      validateRelativePath(filePath)
      const home = await homeDir()
      const fullPath = await join(
        home, '.MoontonClaudeDesktop', 'dynamic-functions',
        functionName.value, filePath
      )
      result = await readTextFile(fullPath)
    } else if (command === 'mcd:writeFile') {
      const filePath = args.path as string
      const content = args.content as string
      validateRelativePath(filePath)
      const home = await homeDir()
      const fullPath = await join(
        home, '.MoontonClaudeDesktop', 'dynamic-functions',
        functionName.value, filePath
      )
      const { writeTextFile } = await import('@tauri-apps/plugin-fs')
      await writeTextFile(fullPath, content)
      result = true
    } else if (command === 'mcd:openExternal') {
      const url = args.url as string
      if (!url) throw new Error('URL is required')
      await invoke('df_open_external', { url })
      result = true
    } else if (command === 'mcd:exec') {
      if (!manifest.value?.capabilities?.exec) {
        throw new Error('exec capability not enabled in manifest')
      }
      const cmdStr = args.command as string
      const cmdArgs = (args.args as string[]) || []
      const options = (args.options as Record<string, unknown>) || {}
      if (!cmdStr) throw new Error('command is required')
      result = await invoke('df_exec', {
        functionName: functionName.value,
        command: cmdStr,
        args: cmdArgs,
        options
      })
    } else if (command === 'mcd:kill') {
      if (!manifest.value?.capabilities?.exec) {
        throw new Error('exec capability not enabled in manifest')
      }
      const pid = args.pid as number
      if (typeof pid !== 'number') throw new Error('pid must be a number')
      await invoke('df_kill', {
        functionName: functionName.value,
        pid
      })
      result = true
    }

    iframe.contentWindow.postMessage({
      type: 'mcd:response',
      id,
      result
    }, '*')
  } catch (e) {
    iframe.contentWindow.postMessage({
      type: 'mcd:response',
      id,
      error: String(e)
    }, '*')
  }
}

/** Compute sandbox attribute based on capabilities */
const sandboxAttr = computed(() => {
  const parts = ['allow-scripts']
  // Only grant allow-same-origin when localStorage is explicitly required
  // Network access is provided via the mcd.backend() bridge, not direct fetch
  if (manifest.value?.capabilities?.localStorage) {
    parts.push('allow-same-origin')
  }
  return parts.join(' ')
})

onMounted(() => {
  window.addEventListener('message', handleMessage)
  loadFunction()
})

onUnmounted(async () => {
  window.removeEventListener('message', handleMessage)
  // Kill spawned background processes
  if (functionName.value) {
    try {
      await invoke('df_cleanup_processes', { functionName: functionName.value })
    } catch (e) {
      console.error('Failed to cleanup spawned processes:', e)
    }
  }
  // Stop backend when window closes
  if (functionName.value && manifest.value?.backend?.enabled) {
    try {
      await dfStore.stopBackend(functionName.value)
    } catch (e) {
      console.error('Failed to stop backend:', e)
    }
  }
})
</script>

<template>
  <div class="dynamic-function-host">
    <!-- Loading state -->
    <div
      v-if="loading"
      class="df-loading"
    >
      <i class="fa-solid fa-spinner fa-spin" />
      <span>加载中...</span>
    </div>

    <!-- Error state -->
    <div
      v-else-if="error"
      class="df-error"
    >
      <i class="fa-solid fa-circle-exclamation" />
      <span>{{ error }}</span>
    </div>

    <!-- iframe -->
    <iframe
      v-show="!loading && !error"
      ref="iframeRef"
      class="df-iframe"
      :sandbox="sandboxAttr"
      title="Dynamic Function"
    />
  </div>
</template>

<style lang="less" scoped>
.dynamic-function-host {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
  overflow: hidden;
}

.df-loading,
.df-error {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #8B8680;
  font-size: 16px;

  i {
    font-size: 32px;
  }
}

.df-error {
  color: #D94F4F;

  i {
    color: #D94F4F;
  }
}

.df-iframe {
  flex: 1;
  width: 100%;
  border: none;
  background: #fff;
}
</style>
