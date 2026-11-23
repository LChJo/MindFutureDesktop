mod app_paths;
mod channels;
mod claude_bot;
mod coworks_config;
mod coworks_fs;
mod coworks_sidecar;
mod devtools;
mod dynamic_function;
mod git;
mod llm_router;
mod marketplace;
mod mcp;
#[cfg(target_os = "windows")]
mod node_pty;
mod nodejs;
mod scheduler_heartbeat;
mod pty;
mod skills;
mod system_aliases;
mod updater;
mod uv;
mod window_state;

use app_paths::{get_app_paths, AppPaths};
use channels::{
    channel_get_config, channel_get_status, channel_save_config, channel_send_message,
    channel_send_platform_message, channel_start, channel_stop, channel_test_connection,
    ChannelManagerState, feishu_add_reaction, feishu_clear_app_reactions, feishu_remove_reaction,
};
use claude_bot::{
    bot_create_bot, bot_delete_bot, bot_ensure_output_directory, bot_filter_message, bot_get_bot,
    bot_get_web_server_status, bot_list_bots, bot_load_conversation, bot_read_claude_md,
    bot_read_output_file, bot_save_conversation, bot_start_web_server, bot_stop_web_server,
    bot_update_bot, bot_write_claude_md, bot_write_output_file, BotWebServerState,
};
use coworks_config::{
    coworks_delete_chat_history, coworks_delete_task_item, coworks_get_config,
    coworks_get_recent_workspaces, coworks_list_chat_histories, coworks_list_tasks,
    coworks_load_chat_history, coworks_load_task, coworks_save_chat_history,
    coworks_save_config, coworks_save_recent_workspaces, coworks_save_task,
    coworks_save_project_session, coworks_list_project_sessions,
    claude_apps_list_conversations, claude_apps_load_conversation,
    claude_apps_save_conversation, claude_apps_delete_conversation,
};
use coworks_fs::{
    coworks_get_file_info, coworks_path_exists, coworks_read_directory, coworks_read_file_text,
};
use coworks_sidecar::{
    coworks_kill_sidecar, coworks_send_to_sidecar, coworks_spawn_sidecar, CoworksSidecarManager,
};
use dynamic_function::{
    df_cleanup_processes, df_delete_function, df_exec, df_kill, df_list_functions,
    df_open_external, df_read_manifest, df_save_function, df_start_backend, df_stop_backend,
    DfProcessManager, DynamicFunctionManager,
};
use git::{check_git_status, install_git_cmd};
use llm_router::{check_llm_router_status, get_ccr_debug_info, restart_llm_router, start_llm_router, stop_llm_router};
use updater::{check_for_update, download_update, get_default_update_url, install_update};
use uv::{check_uv_status, install_uv_cmd};
use marketplace::{
    add_marketplace, disable_plugin,
    enable_plugin, get_all_enabled_plugins, get_all_plugins, get_all_plugins_extended,
    get_installed_plugins, get_marketplace_plugins, get_marketplaces, get_plugin_enabled_status,
    get_plugin_icon, get_plugin_meta, install_plugin, remove_marketplace,
    repair_plugin_registry, uninstall_plugin, update_marketplace,
};
use mcp::{
    add_global_mcp, add_project_mcp, delete_global_mcp, delete_project_mcp, get_all_projects,
    get_global_mcp_servers, get_plugin_mcp_servers, get_projects_with_mcp, toggle_global_mcp,
    toggle_project_mcp,
};
use nodejs::{
    check_claude_code_installed_quick, check_claude_code_status, check_nodejs_status,
    extract_nodejs_cmd, install_claude_code_cmd, install_mcd_cli_cmd,
};
use devtools::{
    devtools_get_session, devtools_list_projects, devtools_list_sessions,
    devtools_start_watching, devtools_stop_watching, DevToolsWatcherState,
};
use skills::{get_local_agents, get_local_skills, get_plugin_agents, get_plugin_skills, open_directory};
use system_aliases::{check_system_aliases_status, register_system_aliases, unregister_system_aliases};
#[cfg(target_os = "windows")]
use node_pty::NodePtyManager;
use pty::PtyManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tauri::State;

#[cfg(target_os = "windows")]
use crate::nodejs::get_nodejs_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAliasConfig {
    pub name: String,
    pub anthropic_model: Option<String>,
    pub anthropic_auth_token: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub custom_env_vars: Option<HashMap<String, String>>,
}

struct AppState {
    pty_manager: Arc<PtyManager>,
    #[cfg(target_os = "windows")]
    node_pty_manager: Option<Arc<NodePtyManager>>,
    app_paths: AppPaths,
}

#[tauri::command]
fn spawn_terminal(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    prompt_config: Option<serde_json::Value>,
    aliases: Option<Vec<ClaudeAliasConfig>>,
) -> Result<String, String> {
    // On Windows, use NodePtyManager if available (for proper Shift+Tab handling)
    #[cfg(target_os = "windows")]
    {
        if let Some(ref node_pty) = state.node_pty_manager {
            log::info!("Using Node.js PTY for terminal (ConPTY backend)");
            return node_pty.spawn(app_handle, cols, rows, cwd, env, prompt_config, aliases);
        }
    }

    // Fall back to portable-pty (macOS or Windows without Node.js)
    #[cfg(target_os = "windows")]
    {
        log::warn!("NodePtyManager unavailable, falling back to portable-pty");
        use tauri::Emitter;
        let _ = app_handle.emit("terminal-degraded", "conpty-fallback");
    }
    // On macOS, prompt_config is used to generate custom zsh prompt
    state.pty_manager.spawn(app_handle, cols, rows, cwd, env, prompt_config, aliases)
}

#[tauri::command]
fn write_terminal(state: State<AppState>, id: String, data: String) -> Result<(), String> {
    // On Windows, check if this terminal is managed by NodePtyManager
    #[cfg(target_os = "windows")]
    {
        if let Some(ref node_pty) = state.node_pty_manager {
            if node_pty.instances.lock().contains_key(&id) {
                return node_pty.write(&id, data.as_bytes());
            }
        }
    }
    state.pty_manager.write(&id, data.as_bytes())
}

#[tauri::command]
fn kill_terminal(state: State<AppState>, id: String) -> Result<(), String> {
    // On Windows, check if this terminal is managed by NodePtyManager
    #[cfg(target_os = "windows")]
    {
        if let Some(ref node_pty) = state.node_pty_manager {
            if node_pty.instances.lock().contains_key(&id) {
                return node_pty.kill(&id);
            }
        }
    }
    state.pty_manager.kill(&id)
}

#[tauri::command]
fn resize_terminal(state: State<AppState>, id: String, cols: u16, rows: u16) -> Result<(), String> {
    // On Windows, check if this terminal is managed by NodePtyManager
    #[cfg(target_os = "windows")]
    {
        if let Some(ref node_pty) = state.node_pty_manager {
            if node_pty.instances.lock().contains_key(&id) {
                return node_pty.resize(&id, cols, rows);
            }
        }
    }
    state.pty_manager.resize(&id, cols, rows)
}

/// Check if the app is running from a read-only DMG mount (macOS).
/// Returns true if the user should drag the app to Applications first.
#[tauri::command]
fn check_running_from_dmg() -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe) = std::env::current_exe() {
            if let Ok(resolved) = exe.canonicalize() {
                return resolved.to_string_lossy().starts_with("/Volumes/");
            }
        }
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[tauri::command]
fn get_app_paths_cmd(state: State<AppState>) -> AppPaths {
    AppPaths {
        app_data: state.app_paths.app_data.clone(),
        config: state.app_paths.config.clone(),
        logs: state.app_paths.logs.clone(),
        cache: state.app_paths.cache.clone(),
        workspace: state.app_paths.workspace.clone(),
    }
}

#[tauri::command]
fn app_log(scope: String, level: Option<String>, message: String) -> Result<(), String> {
    let scope = scope.trim();
    let prefix = if scope.is_empty() {
        "[frontend]"
    } else {
        ""
    };
    let rendered = if scope.is_empty() {
        format!("{} {}", prefix, message)
    } else {
        format!("[frontend::{}] {}", scope, message)
    };

    match level.as_deref().unwrap_or("info") {
        "error" => log::error!("{}", rendered),
        "warn" => log::warn!("{}", rendered),
        "debug" => log::debug!("{}", rendered),
        _ => log::info!("{}", rendered),
    }
    Ok(())
}

#[tauri::command]
fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(&path).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path().to_string_lossy().to_string();

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        files.push(FileEntry {
            name,
            path: file_path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
        });
    }

    // Sort: directories first, then by name
    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(files)
}

#[tauri::command]
fn open_file_default(path: String) -> Result<(), String> {
    opener::open(&path).map_err(|e| format!("Failed to open file: {}", e))
}

#[tauri::command]
fn reveal_in_explorer(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        if path.is_file() {
            // Select the file in explorer
            Command::new("explorer")
                .args(["/select,", &path.to_string_lossy()])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| format!("Failed to open explorer: {}", e))?;
        } else if path.is_dir() {
            // Open the directory
            Command::new("explorer")
                .arg(&path.to_string_lossy().to_string())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| format!("Failed to open explorer: {}", e))?;
        } else {
            return Err("Path does not exist".to_string());
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        // Try common file managers
        let parent = path.parent().unwrap_or(path);
        Command::new("xdg-open")
            .arg(&parent.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn delete_file(path: String) -> Result<(), String> {
    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to get file info: {}", e))?;

    if metadata.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete file: {}", e))
    }
}

/// Font information for system font detection
#[derive(serde::Serialize)]
pub struct FontInfo {
    pub name: String,
    pub is_monospace: bool,
}

/// Get system fonts, primarily for terminal font selection
#[tauri::command]
fn get_system_fonts() -> Vec<FontInfo> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut seen = std::collections::HashSet::new();
    let mut fonts = Vec::new();

    for face in db.faces() {
        let family = face.families.first()
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        if !family.is_empty() && seen.insert(family.clone()) {
            fonts.push(FontInfo {
                name: family,
                is_monospace: face.monospaced,
            });
        }
    }

    // Sort by font name
    fonts.sort_by(|a, b| a.name.cmp(&b.name));
    fonts
}

/// Git status information for a directory
#[derive(serde::Serialize)]
struct GitStatus {
    is_git_repo: bool,
    branch: Option<String>,
    has_changes: bool,
    ahead: u32,
    behind: u32,
}

/// Resolve git executable: bundled path first, then fall back to bare "git"
fn resolve_git_executable() -> String {
    if let Some(bundled) = git::get_git_exe_path() {
        if bundled.exists() {
            return bundled.to_string_lossy().to_string();
        }
    }
    "git".to_string()
}

/// Get Git status for a directory (branch, changes, etc.)
#[tauri::command]
fn get_git_status(path: String) -> GitStatus {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let path = std::path::Path::new(&path);

    // Check if this is a git repo
    let git_dir = path.join(".git");
    let is_git_repo = if git_dir.exists() {
        true
    } else {
        // Check parent directories for .git
        let mut current = path.parent();
        let mut found = false;
        while let Some(p) = current {
            if p.join(".git").exists() {
                found = true;
                break;
            }
            current = p.parent();
        }
        found
    };

    if !is_git_repo {
        return GitStatus {
            is_git_repo: false,
            branch: None,
            has_changes: false,
            ahead: 0,
            behind: 0,
        };
    }

    let git_exe = resolve_git_executable();

    // Get current branch
    let mut cmd = Command::new(&git_exe);
    cmd.args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let branch = cmd
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        });

    // Check for uncommitted changes
    let mut cmd = Command::new(&git_exe);
    cmd.args(["status", "--porcelain"])
        .current_dir(path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let has_changes = cmd
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    // Get ahead/behind counts
    let mut cmd = Command::new(&git_exe);
    cmd.args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let (ahead, behind) = cmd
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let s = String::from_utf8(output.stdout).ok()?;
                let parts: Vec<&str> = s.trim().split('\t').collect();
                if parts.len() == 2 {
                    let a = parts[0].parse().unwrap_or(0);
                    let b = parts[1].parse().unwrap_or(0);
                    Some((a, b))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or((0, 0));

    GitStatus {
        is_git_repo: true,
        branch,
        has_changes,
        ahead,
        behind,
    }
}

/// Update shell configuration file (.zshrc) with environment variables
/// This ensures env vars persist across terminal sessions
#[tauri::command]
fn update_shell_env(key: String, value: Option<String>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Keep imports minimal to avoid unused warnings.

        let home = std::env::var("HOME").map_err(|_| "Failed to get HOME directory")?;
        let zshrc_path = std::path::Path::new(&home).join(".zshrc");

        // Read existing content
        let content = if zshrc_path.exists() {
            fs::read_to_string(&zshrc_path)
                .map_err(|e| format!("Failed to read .zshrc: {}", e))?
        } else {
            String::new()
        };

        // Build new content
        let mut new_lines = Vec::new();
        let export_line = format!("export {}=\"{}\"", key, value.as_ref().unwrap_or(&String::new()));
        let mut found = false;

        for line in content.lines() {
            // Check if this line exports the same variable
            if line.trim().starts_with(&format!("export {}=", key)) {
                if value.is_some() {
                    // Replace with new value
                    new_lines.push(export_line.clone());
                    found = true;
                }
                // If value is None, skip this line (remove the export)
            } else {
                new_lines.push(line.to_string());
            }
        }

        // If variable wasn't found and we have a value, append it
        if !found {
            if let Some(_) = value {
                new_lines.push(export_line);
            }
        }

        // Write back to file
        fs::write(&zshrc_path, new_lines.join("\n") + "\n")
            .map_err(|e| format!("Failed to write .zshrc: {}", e))?;

        log::info!("Updated .zshrc: {} = {:?}", key, value);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, just return Ok (no-op)
        let _ = (key, value); // Suppress unused variable warnings
        Ok(())
    }
}

/// 将指定环境变量写入 Windows 用户环境变量 (HKCU\Environment)
/// 使用 PowerShell [System.Environment]::SetEnvironmentVariable 自动广播 WM_SETTINGCHANGE，
/// 使新开的程序无需注销即可读取新变量。
#[tauri::command]
fn sync_env_vars_to_windows_user(vars: HashMap<String, String>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        if vars.is_empty() {
            return Ok(());
        }

        let ps_lines: Vec<String> = vars
            .iter()
            .filter(|(k, v)| !k.is_empty() && !v.is_empty())
            .map(|(k, v)| {
                let ek = k.replace('\'', "''");
                let ev = v.replace('\'', "''");
                format!(
                    "[System.Environment]::SetEnvironmentVariable('{}', '{}', 'User')",
                    ek, ev
                )
            })
            .collect();

        if ps_lines.is_empty() {
            return Ok(());
        }

        let script = ps_lines.join("; ");

        let output = std::process::Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("PowerShell error: {}", stderr));
        }

        log::info!("Synced {} env vars to Windows user environment", vars.len());
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = vars;
        Err("Only supported on Windows".to_string())
    }
}

/// 解析 shell 配置文件中的 export 语句
/// 支持：export KEY="value"、export KEY='value'、export KEY=value
#[cfg(target_os = "macos")]
fn parse_shell_exports(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            if let Some(eq_pos) = rest.find('=') {
                let key = rest[..eq_pos].trim();
                if key.is_empty() || key.contains(' ') {
                    continue;
                }
                let raw_val = rest[eq_pos + 1..].trim();
                let value = if (raw_val.starts_with('"') && raw_val.ends_with('"'))
                    || (raw_val.starts_with('\'') && raw_val.ends_with('\''))
                {
                    raw_val[1..raw_val.len() - 1].to_string()
                } else {
                    raw_val.to_string()
                };
                map.insert(key.to_string(), value);
            }
        }
    }
    map
}

/// Read system environment variables by keys.
/// Returns a map of key -> value (None if the variable is not set).
/// On macOS, falls back to parsing .zshrc for variables not found in the process environment.
#[tauri::command]
fn get_system_env_vars(keys: Vec<String>) -> HashMap<String, Option<String>> {
    #[allow(unused_mut)]
    let mut result: HashMap<String, Option<String>> = keys.iter()
        .map(|key| (key.clone(), std::env::var(key).ok()))
        .collect();

    #[cfg(target_os = "macos")]
    {
        let missing: Vec<&String> = keys.iter()
            .filter(|k| result.get(*k).and_then(|v| v.as_ref()).map_or(true, |v: &String| v.is_empty()))
            .collect();

        if !missing.is_empty() {
            if let Ok(home) = std::env::var("HOME") {
                let zshrc_path = std::path::Path::new(&home).join(".zshrc");
                if let Ok(content) = fs::read_to_string(&zshrc_path) {
                    let zshrc_vars = parse_shell_exports(&content);
                    for key in missing {
                        if let Some(val) = zshrc_vars.get(key.as_str()) {
                            result.insert(key.clone(), Some(val.clone()));
                        }
                    }
                }
            }
        }
    }

    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pty_manager = Arc::new(PtyManager::new());

    // Initialize app paths and ensure directories exist
    let app_paths = get_app_paths().expect("Failed to initialize app paths");
    let log_dir = std::path::PathBuf::from(&app_paths.logs);

    // Initialize Node.js PTY manager on Windows if Node.js is available
    #[cfg(target_os = "windows")]
    let node_pty_manager: Option<Arc<NodePtyManager>> = {
        // Get Node.js path
        let node_path = get_nodejs_path();

        // Get pty-server.js path from resources
        // During development: src-tauri/resources/nodejs-windows/pty-server.js
        // In production: bundled with the app (next to executable)
        let pty_server_path = {
            // Try development path first
            let dev_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("nodejs-windows");

            if dev_dir.join("node_modules").exists() {
                Some(dev_dir.join("pty-server.js").to_string_lossy().to_string())
            } else {
                // Try production path (relative to executable)
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        let prod_dir = exe_dir.join("resources").join("nodejs-windows");
                        if prod_dir.join("node_modules").exists() {
                            Some(prod_dir.join("pty-server.js").to_string_lossy().to_string())
                        } else {
                            log::warn!("Production nodejs-windows resources not found at: {:?}", prod_dir);
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        match (node_path, pty_server_path) {
            (Some(node), Some(server)) => {
                let node_str = node.to_string_lossy().to_string();
                log::info!("Initializing NodePtyManager with node: {}, server: {}", node_str, server);
                Some(Arc::new(NodePtyManager::new(node_str, server)))
            }
            (node, server) => {
                log::info!(
                    "NodePtyManager not available - node: {:?}, pty-server: {:?}",
                    node.is_some(),
                    server.is_some()
                );
                None
            }
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            pty_manager: pty_manager.clone(),
            #[cfg(target_os = "windows")]
            node_pty_manager,
            app_paths,
        })
        .manage(CoworksSidecarManager::new())
        .manage(DynamicFunctionManager::new())
        .manage(DfProcessManager::new())
        .manage(BotWebServerState::default())
        .manage(ChannelManagerState::new())
        .manage(std::sync::Mutex::new(DevToolsWatcherState::default()))
        .setup(move |app| {
            use tauri::Manager;

            // Configure logging to AppData logs directory
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Folder {
                            path: log_dir.clone(),
                            file_name: Some("moonton-claude".to_string()),
                        },
                    ))
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout,
                    ))
                    .build(),
            )?;

            log::info!("Moonton Claude Desktop started");

            // Restore window state and show window
            if let Some(window) = app.get_webview_window("main") {
                // Restore saved state if exists
                if let Some(state) = window_state::load() {
                    if let Err(e) = state.apply_to_window(&window) {
                        log::warn!("Failed to restore window state: {}", e);
                    }
                }

                // Show window after state is restored
                if let Err(e) = window.show() {
                    log::warn!("Failed to show window: {}", e);
                }

                // Listen for window close event to save state
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // Save window state before closing
                        if let Ok(state) = window_state::WindowState::from_window(&window_clone) {
                            if let Err(e) = window_state::save(&state) {
                                log::warn!("Failed to save window state: {}", e);
                            }
                        }
                    }
                });
            }

            // Start scheduler heartbeat for frontend bot timers (immune to WebView throttling)
            scheduler_heartbeat::start_heartbeat(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            spawn_terminal,
            write_terminal,
            kill_terminal,
            resize_terminal,
            check_running_from_dmg,
            get_app_paths_cmd,
            get_marketplaces,
            add_marketplace,
            update_marketplace,
            remove_marketplace,
            get_marketplace_plugins,
            get_all_plugins,
            get_installed_plugins,
            install_plugin,
            uninstall_plugin,
            enable_plugin,
            disable_plugin,
            get_plugin_enabled_status,
            get_all_enabled_plugins,
            repair_plugin_registry,
            get_plugin_meta,
            get_all_plugins_extended,
            get_plugin_icon,
            get_local_skills,
            get_local_agents,
            get_plugin_skills,
            get_plugin_agents,
            open_directory,
            // File operations
            list_directory,
            open_file_default,
            reveal_in_explorer,
            delete_file,
            get_git_status,
            // Shell environment management
            update_shell_env,
            get_system_env_vars,
            sync_env_vars_to_windows_user,
            // System-level alias registration
            check_system_aliases_status,
            register_system_aliases,
            unregister_system_aliases,
            // MCP management
            get_global_mcp_servers,
            get_plugin_mcp_servers,
            get_projects_with_mcp,
            get_all_projects,
            toggle_global_mcp,
            toggle_project_mcp,
            delete_global_mcp,
            delete_project_mcp,
            add_global_mcp,
            add_project_mcp,
            // Node.js management
            check_nodejs_status,
            extract_nodejs_cmd,
            check_claude_code_installed_quick,
            check_claude_code_status,
            install_claude_code_cmd,
            install_mcd_cli_cmd,
            // Git management
            check_git_status,
            install_git_cmd,
            // UV management
            check_uv_status,
            install_uv_cmd,
            // Font detection
            get_system_fonts,
            // Coworks - File system operations
            coworks_read_directory,
            coworks_read_file_text,
            coworks_path_exists,
            coworks_get_file_info,
            // Coworks - Chat history
            coworks_list_chat_histories,
            coworks_load_chat_history,
            coworks_save_chat_history,
            coworks_delete_chat_history,
            // Coworks - Tasks
            coworks_list_tasks,
            coworks_load_task,
            coworks_save_task,
            coworks_delete_task_item,
            // Coworks - Workspaces and config
            coworks_get_recent_workspaces,
            coworks_save_recent_workspaces,
            coworks_get_config,
            coworks_save_config,
            // Coworks - Project sessions (Code mode SDK session persistence)
            coworks_save_project_session,
            coworks_list_project_sessions,
            // Coworks - Sidecar management
            coworks_spawn_sidecar,
            coworks_send_to_sidecar,
            coworks_kill_sidecar,
            // LLM Router management
            check_llm_router_status,
            start_llm_router,
            stop_llm_router,
            restart_llm_router,
            get_ccr_debug_info,
            // Auto-update
            get_default_update_url,
            check_for_update,
            download_update,
            install_update,
            // Dynamic functions
            df_list_functions,
            df_read_manifest,
            df_save_function,
            df_start_backend,
            df_stop_backend,
            df_delete_function,
            df_open_external,
            df_exec,
            df_kill,
            df_cleanup_processes,
            // Claude Apps conversations
            claude_apps_list_conversations,
            claude_apps_load_conversation,
            claude_apps_save_conversation,
            claude_apps_delete_conversation,
            // DevTools session analyzer
            devtools_list_projects,
            devtools_list_sessions,
            devtools_get_session,
            devtools_start_watching,
            devtools_stop_watching,
            // Claude Bot management
            bot_list_bots,
            bot_get_bot,
            bot_create_bot,
            bot_update_bot,
            bot_delete_bot,
            bot_save_conversation,
            bot_load_conversation,
            bot_read_output_file,
            bot_write_output_file,
            bot_ensure_output_directory,
            bot_read_claude_md,
            bot_write_claude_md,
            bot_start_web_server,
            bot_stop_web_server,
            bot_get_web_server_status,
            bot_filter_message,
            // Channel management
            channel_get_config,
            channel_save_config,
            channel_test_connection,
            channel_start,
            channel_stop,
            channel_get_status,
            channel_send_message,
            channel_send_platform_message,
            // Feishu reaction management
            feishu_clear_app_reactions,
            feishu_add_reaction,
            feishu_remove_reaction,
            app_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
