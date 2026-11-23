# API 契约

> 由 wiki_gen 生成。

## API 契约

### claude-apps

- **df_list_functions** `[inbound]`
  - List all installed dynamic functions
- **df_create_function** `[inbound]`
  - Create a new dynamic function from template
- **df_start_backend** `[inbound]`
  - Start the backend process for a dynamic function
- **df_stop_backend** `[inbound]`
  - Stop the backend process
- **mcd.exec** `[inbound]`
  - Execute shell command from within a dynamic function (sandboxed)
- **mcd.kill** `[inbound]`
  - Kill a running process started by mcd.exec
- **mcd.openExternal** `[inbound]`
  - Open URL or file with system default handler

### claude-bot

- **createBot** `[inbound]`
  - Create a new autonomous bot
- **startBot** `[inbound]`
  - Start bot execution
- **channel_send** `[outbound]`
  - Send message to external channel

### coworks

- **startSession** `[inbound]`
  - Start a new Cowork session
- **sendMessage** `[inbound]`
  - Send user message to Claude
- **sidecar_spawn** `[outbound]`
  - Spawn Claude Agent SDK sidecar process

### desktop

- **openWindow** `[inbound]`
  - Open a new window for the given component
- **closeWindow** `[inbound]`
  - Close and destroy a window by ID
- **focusWindow** `[inbound]`
  - Bring window to front and set focus
- **moveIcon** `[inbound]`
  - Drag-drop repositioning of desktop icon

### devtools

- **getSessionMetrics** `[inbound]`
  - Get metrics for a Cowork session

### llm-router

- **getRouterConfig** `[inbound]`
  - Get current LLM routing config
- **setRouterConfig** `[inbound]`
  - Update LLM routing config

### marketplace

- **searchPlugins** `[inbound]`
  - Search marketplace catalog
- **installPlugin** `[inbound]`
  - Install plugin from source
- **uninstallPlugin** `[inbound]`
  - Remove installed plugin

### mcp

- **listMcpServers** `[inbound]`
  - List configured MCP servers
- **addMcpServer** `[inbound]`
  - Add MCP server config
- **removeMcpServer** `[inbound]`
  - Remove MCP server config

### settings

- **getSettings** `[inbound]`
  - Load application settings
- **saveSettings** `[inbound]`
  - Persist settings changes
- **getWindowState** `[inbound]`
  - Restore window position and size

### system

- **checkDependencies** `[inbound]`
  - Check all system dependencies
- **checkForUpdate** `[inbound]`
  - Check for application updates
- **installUpdate** `[inbound]`
  - Download and install update

### terminal

- **pty_spawn** `[inbound]`
  - Spawn a new PTY session
- **pty_write** `[inbound]`
  - Write data to PTY stdin
- **pty_resize** `[inbound]`
  - Resize PTY dimensions
- **pty_kill** `[inbound]`
  - Kill PTY session
