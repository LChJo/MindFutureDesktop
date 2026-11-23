//! Claude Bot management
//!
//! Manages bot definitions, conversations, output files, and a lightweight
//! status dashboard web server.  All data is stored under
//! `~/.MoontonClaudeDesktop/bots/`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read as IoRead, Write as IoWrite};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

// ---------------------------------------------------------------------------
// Predefined palette for new bots
// ---------------------------------------------------------------------------

const BOT_COLORS: &[&str] = &[
    "#00ff88", "#ff6b6b", "#4ecdc4", "#ffe66d", "#a29bfe", "#fd79a8",
];

// ---------------------------------------------------------------------------
// Data directory helpers (follows coworks_config.rs pattern)
// ---------------------------------------------------------------------------

/// Get the bots data directory (`~/.MoontonClaudeDesktop/bots/`)
fn get_bots_data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    let data_dir = home.join(".MoontonClaudeDesktop").join("bots");
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create bots data directory: {}", e))?;
    }
    Ok(data_dir)
}

/// Get the directory for a specific bot (`~/.MoontonClaudeDesktop/bots/{botId}/`)
fn get_bot_dir(bot_id: &str) -> Result<PathBuf, String> {
    let base = get_bots_data_dir()?;
    let bot_dir = base.join(bot_id);
    if !bot_dir.exists() {
        fs::create_dir_all(&bot_dir)
            .map_err(|e| format!("Failed to create bot directory: {}", e))?;
    }
    Ok(bot_dir)
}

/// Current timestamp in milliseconds since UNIX epoch.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Lightweight entry stored in the top-level `index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotIndexItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "triggerType")]
    pub trigger_type: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    #[serde(rename = "lastActiveAt", skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<u64>,
}

/// Full bot definition persisted in `{botId}/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotData {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "triggerType")]
    pub trigger_type: String,
    pub status: String,
    pub model: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "spriteVariant")]
    pub sprite_variant: u32,
    pub color: String,
    #[serde(rename = "workDir")]
    pub work_dir: String,
    #[serde(rename = "sidecarSessionId", skip_serializing_if = "Option::is_none")]
    pub sidecar_session_id: Option<String>,
    #[serde(rename = "sdkSessionId", skip_serializing_if = "Option::is_none")]
    pub sdk_session_id: Option<String>,
    #[serde(rename = "scheduleConfig", skip_serializing_if = "Option::is_none")]
    pub schedule_config: Option<serde_json::Value>,
    #[serde(rename = "proactiveConfig", skip_serializing_if = "Option::is_none")]
    pub proactive_config: Option<serde_json::Value>,
    #[serde(rename = "totalInteractions")]
    pub total_interactions: u64,
    #[serde(rename = "lastActiveAt", skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<u64>,
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Message filter configuration (Haiku-based pre-filter for channel messages).
    #[serde(rename = "messageFilter", skip_serializing_if = "Option::is_none")]
    pub message_filter: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Index helpers
// ---------------------------------------------------------------------------

fn read_index() -> Result<Vec<BotIndexItem>, String> {
    let index_path = get_bots_data_dir()?.join("index.json");
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read bot index: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse bot index: {}", e))
}

/// Write content atomically: write to a sibling .tmp file, sync, then rename.
/// Prevents partial/corrupted files if the process crashes mid-write.
/// On Windows NTFS (same-volume rename) this is atomic; FAT32/exFAT do not
/// guarantee atomicity but are rarely used for the app data directory.
fn atomic_write(path: &std::path::Path, content: &str) -> Result<(), String> {
    // Append ".tmp" to the full filename (e.g. config.json → config.json.tmp)
    // to avoid with_extension() stripping the existing extension.
    let tmp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    let write_result = (|| {
        let mut f = fs::File::create(&tmp)
            .map_err(|e| format!("Failed to create temp file {:?}: {}", tmp, e))?;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        f.sync_all()
            .map_err(|e| format!("Failed to sync temp file: {}", e))?;
        Ok::<(), String>(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp); // best-effort cleanup on partial write
        return write_result;
    }
    fs::rename(&tmp, path)
        .map_err(|e| format!("Failed to rename {:?} -> {:?}: {}", tmp, path, e))
}

fn write_index(index: &[BotIndexItem]) -> Result<(), String> {
    let index_path = get_bots_data_dir()?.join("index.json");
    let content = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize bot index: {}", e))?;
    atomic_write(&index_path, &content)
}

fn read_bot_config(bot_id: &str) -> Result<BotData, String> {
    let config_path = get_bot_dir(bot_id)?.join("config.json");
    if !config_path.exists() {
        return Err(format!("Bot not found: {}", bot_id));
    }
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read bot config: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse bot config: {}", e))
}

fn write_bot_config(bot: &BotData) -> Result<(), String> {
    let config_path = get_bot_dir(&bot.id)?.join("config.json");
    let content = serde_json::to_string_pretty(bot)
        .map_err(|e| format!("Failed to serialize bot config: {}", e))?;
    atomic_write(&config_path, &content)
}

// ---------------------------------------------------------------------------
// Path traversal protection
// ---------------------------------------------------------------------------

/// Ensure `child` is strictly inside `parent` after canonicalization.
/// Both paths must exist on disk (create the parent dir first if needed).
fn ensure_within(parent: &std::path::Path, child: &std::path::Path) -> Result<(), String> {
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to resolve parent path: {}", e))?;
    let canonical_child = child
        .canonicalize()
        .map_err(|e| format!("Failed to resolve child path: {}", e))?;
    if !canonical_child.starts_with(&canonical_parent) {
        return Err("Path traversal blocked: resolved path is outside the allowed directory".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Default output template
// ---------------------------------------------------------------------------

/// Ensure the `output/` directory exists for a bot.
fn ensure_output_dir(bot_id: &str) -> Result<PathBuf, String> {
    let bot_dir = get_bot_dir(bot_id)?;
    let output_dir = bot_dir.join("output");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    Ok(output_dir)
}

// ============================================================================
// Tauri commands — CRUD
// ============================================================================

/// List all bots (index only).
/// List all bots from index. Entries whose config.json is missing are skipped
/// (logged as warnings) but intentionally NOT removed from index.json — auto-pruning
/// could permanently erase bots after a disk hiccup or cross-machine sync.
#[tauri::command]
pub fn bot_list_bots() -> Result<Vec<BotIndexItem>, String> {
    let index = read_index()?;
    let bots_dir = get_bots_data_dir()?;

    let mut valid = Vec::new();
    for item in index {
        if bots_dir.join(&item.id).join("config.json").exists() {
            valid.push(item);
        } else {
            log::warn!("bot_list_bots: config.json missing for bot {} ({}), skipping (not pruned)", item.name, item.id);
        }
    }

    Ok(valid)
}

/// Get full bot data by ID.
/// If workDir points to a non-existent path (e.g. copied from another machine),
/// auto-corrects it to the correct path on this machine and persists the fix.
#[tauri::command]
pub fn bot_get_bot(bot_id: String) -> Result<BotData, String> {
    let mut bot = read_bot_config(&bot_id)?;

    // Auto-fix workDir if it doesn't exist on this machine (cross-machine migration)
    let expected_work_dir = get_bot_dir(&bot_id)?
        .to_string_lossy()
        .to_string();
    if bot.work_dir != expected_work_dir && !std::path::Path::new(&bot.work_dir).exists() {
        log::info!(
            "bot_get_bot: correcting workDir for {} from {:?} to {:?}",
            bot.name, bot.work_dir, expected_work_dir
        );
        bot.work_dir = expected_work_dir;
        let _ = write_bot_config(&bot);
    }

    Ok(bot)
}

/// Create a new bot.  Returns the full `BotData`.
#[tauri::command]
pub fn bot_create_bot(
    name: String,
    description: String,
    trigger_type: String,
    model: String,
    system_prompt: String,
    sprite_variant: Option<u32>,
    color: Option<String>,
    schedule_config: Option<serde_json::Value>,
    proactive_config: Option<serde_json::Value>,
) -> Result<BotData, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now_millis();

    // Pick sprite variant: supplied or random 0-5
    let sprite = sprite_variant.unwrap_or_else(|| {
        use rand::Rng;
        rand::thread_rng().gen_range(0..6)
    });

    // Pick colour: supplied or random from palette
    let chosen_color = color.unwrap_or_else(|| {
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..BOT_COLORS.len());
        BOT_COLORS[idx].to_string()
    });

    // Work directory is the bot's own directory
    let work_dir = get_bot_dir(&id)?
        .to_string_lossy()
        .to_string();

    // Ensure workspace subdirectory exists
    let workspace_dir = get_bot_dir(&id)?.join("workspace");
    if !workspace_dir.exists() {
        fs::create_dir_all(&workspace_dir)
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
    }

    let bot = BotData {
        id: id.clone(),
        name: name.clone(),
        description,
        trigger_type: trigger_type.clone(),
        status: "idle".to_string(),
        model,
        created_at: ts,
        updated_at: ts,
        system_prompt,
        sprite_variant: sprite,
        color: chosen_color,
        work_dir,
        sidecar_session_id: None,
        sdk_session_id: None,
        schedule_config,
        proactive_config,
        total_interactions: 0,
        last_active_at: None,
        error_message: None,
        message_filter: None,
    };

    // Persist config
    write_bot_config(&bot)?;

    // Create output template
    ensure_output_dir(&id)?;

    // Create CLAUDE.md for persistent memory
    create_claude_md(&id, &bot.name, &bot.trigger_type, &bot.description)?;

    // Update index
    let mut index = read_index()?;
    index.push(BotIndexItem {
        id: id.clone(),
        name,
        trigger_type,
        status: "idle".to_string(),
        created_at: ts,
        updated_at: ts,
        last_active_at: None,
    });
    index.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_index(&index)?;

    Ok(bot)
}

/// Merge partial JSON updates into an existing bot config.
#[tauri::command]
pub fn bot_update_bot(bot_id: String, updates: String) -> Result<BotData, String> {
    let mut bot = read_bot_config(&bot_id)?;

    let patch: serde_json::Value = serde_json::from_str(&updates)
        .map_err(|e| format!("Invalid update JSON: {}", e))?;

    // Serialize current bot to a mutable Value, merge, then deserialize back.
    let mut base: serde_json::Value = serde_json::to_value(&bot)
        .map_err(|e| format!("Failed to serialize bot for merge: {}", e))?;

    if let (Some(base_obj), Some(patch_obj)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }

    // Always bump updatedAt
    if let Some(obj) = base.as_object_mut() {
        obj.insert("updatedAt".to_string(), serde_json::json!(now_millis()));
    }

    bot = serde_json::from_value(base)
        .map_err(|e| format!("Failed to deserialize merged bot: {}", e))?;

    write_bot_config(&bot)?;

    // Sync index
    let mut index = read_index()?;
    if let Some(entry) = index.iter_mut().find(|e| e.id == bot_id) {
        entry.name = bot.name.clone();
        entry.trigger_type = bot.trigger_type.clone();
        entry.status = bot.status.clone();
        entry.updated_at = bot.updated_at;
        entry.last_active_at = bot.last_active_at;
    }
    index.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_index(&index)?;

    Ok(bot)
}

/// Delete a bot — removes the index entry first (so UI stops referencing it),
/// then attempts to delete the directory (failure is non-fatal).
#[tauri::command]
pub fn bot_delete_bot(bot_id: String) -> Result<(), String> {
    // 1. Remove from index first (ensures UI consistency even if dir deletion fails)
    let mut index = read_index()?;
    index.retain(|e| e.id != bot_id);
    write_index(&index)?;

    // 2. Attempt to delete the directory (non-fatal if locked by sidecar)
    let bot_dir = get_bots_data_dir()?.join(&bot_id);
    if bot_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&bot_dir) {
            log::warn!("Failed to delete bot directory for {}: {} (will be cleaned up later)", bot_id, e);
        }
    }

    Ok(())
}

// ============================================================================
// Tauri commands — Conversation persistence
// ============================================================================

/// Save conversation JSON to `{botId}/conversation.json`.
#[tauri::command]
pub fn bot_save_conversation(bot_id: String, conversation: String) -> Result<(), String> {
    // Validate that conversation is valid JSON
    let _: serde_json::Value = serde_json::from_str(&conversation)
        .map_err(|e| format!("Invalid conversation JSON: {}", e))?;

    let conv_path = get_bot_dir(&bot_id)?.join("conversation.json");
    fs::write(&conv_path, &conversation)
        .map_err(|e| format!("Failed to write conversation: {}", e))
}

/// Load conversation JSON from `{botId}/conversation.json`.
/// Returns the raw JSON string (the frontend deserializes it).
#[tauri::command]
pub fn bot_load_conversation(bot_id: String) -> Result<String, String> {
    let conv_path = get_bot_dir(&bot_id)?.join("conversation.json");
    if !conv_path.exists() {
        return Ok("[]".to_string());
    }
    fs::read_to_string(&conv_path)
        .map_err(|e| format!("Failed to read conversation: {}", e))
}

// ============================================================================
// Tauri commands — Output file I/O
// ============================================================================

/// Read a file from the bot's `output/` directory.
/// `file_path` is relative to `output/` (e.g. `"index.html"`).
#[tauri::command]
pub fn bot_read_output_file(bot_id: String, file_path: String) -> Result<String, String> {
    // Basic path-component validation
    if file_path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    if file_path.starts_with('/') || file_path.starts_with('\\') {
        return Err("Absolute paths not allowed".to_string());
    }
    if file_path.len() >= 2 && file_path.as_bytes()[1] == b':' {
        return Err("Absolute paths not allowed".to_string());
    }
    if file_path.contains('\0') {
        return Err("Null bytes not allowed in path".to_string());
    }

    let output_dir = get_bot_dir(&bot_id)?.join("output");
    let target = output_dir.join(&file_path);

    if !target.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Canonicalize and verify containment
    ensure_within(&output_dir, &target)?;

    fs::read_to_string(&target)
        .map_err(|e| format!("Failed to read output file: {}", e))
}

/// Write a file into the bot's `output/` directory.
/// `file_path` is relative to `output/` (e.g. `"index.html"`).
#[tauri::command]
pub fn bot_write_output_file(
    bot_id: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    // Basic path-component validation
    if file_path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    if file_path.starts_with('/') || file_path.starts_with('\\') {
        return Err("Absolute paths not allowed".to_string());
    }
    if file_path.len() >= 2 && file_path.as_bytes()[1] == b':' {
        return Err("Absolute paths not allowed".to_string());
    }
    if file_path.contains('\0') {
        return Err("Null bytes not allowed in path".to_string());
    }

    let output_dir = get_bot_dir(&bot_id)?.join("output");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let target = output_dir.join(&file_path);

    // Create intermediate directories if needed
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directories: {}", e))?;
        }
    }

    // Write the file first so it exists for canonicalization
    fs::write(&target, &content)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    // Verify containment after write
    if let Err(e) = ensure_within(&output_dir, &target) {
        // Roll back the write
        let _ = fs::remove_file(&target);
        return Err(e);
    }

    Ok(())
}

/// Ensure the bot's `output/` directory exists with default template files.
/// Returns the absolute path to the output directory.
#[tauri::command]
pub fn bot_ensure_output_directory(bot_id: String) -> Result<String, String> {
    let output_dir = ensure_output_dir(&bot_id)?;
    Ok(output_dir.to_string_lossy().to_string())
}

// ============================================================================
// CLAUDE.md — persistent memory for bots
// ============================================================================

/// Create a CLAUDE.md file in the bot's root directory for SDK to load.
fn create_claude_md(
    bot_id: &str,
    name: &str,
    _trigger_type: &str,
    description: &str,
) -> Result<(), String> {
    let bot_dir = get_bot_dir(bot_id)?;
    let claude_md_path = bot_dir.join("CLAUDE.md");

    // Don't overwrite if it already exists
    if claude_md_path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"# {name} 记忆

## 身份
{description}

## 用户画像
<!-- 用户的沟通风格、语言偏好、专业领域、常见需求模式 -->

## 工具与工作流偏好
<!-- 用户偏好的工具链、常用命令、代码风格、框架选择 -->

## 关键知识
<!-- 用户项目的架构决策、重要约定、领域知识、API 用法 -->

## 错误规避
<!-- 曾经犯过的错误、用户纠正过的行为、需要避免的陷阱 -->

## 近期摘要
<!-- 最近 10 条对话摘要，按时间倒序，每条包含日期和要点 -->
"#,
        name = name,
        description = description,
    );

    fs::write(&claude_md_path, content)
        .map_err(|e| format!("Failed to write CLAUDE.md: {}", e))
}

/// Read the bot's CLAUDE.md file.
#[tauri::command]
pub fn bot_read_claude_md(bot_id: String) -> Result<String, String> {
    let bot_dir = get_bot_dir(&bot_id)?;
    let claude_md_path = bot_dir.join("CLAUDE.md");
    if !claude_md_path.exists() {
        return Err(format!("CLAUDE.md not found for bot: {}", bot_id));
    }
    fs::read_to_string(&claude_md_path)
        .map_err(|e| format!("Failed to read CLAUDE.md: {}", e))
}

/// Write content to the bot's CLAUDE.md file.
#[tauri::command]
pub fn bot_write_claude_md(bot_id: String, content: String) -> Result<(), String> {
    let bot_dir = get_bot_dir(&bot_id)?;
    let claude_md_path = bot_dir.join("CLAUDE.md");
    fs::write(&claude_md_path, &content)
        .map_err(|e| format!("Failed to write CLAUDE.md: {}", e))
}

// ============================================================================
// Web server — lightweight bot status dashboard
// ============================================================================

/// Handle held inside the Mutex so we can stop the server later.
struct BotWebServerHandle {
    port: u16,
    shutdown_flag: Arc<AtomicBool>,
}

/// Managed Tauri state for the bot web server.
pub struct BotWebServerState {
    server_handle: Mutex<Option<BotWebServerHandle>>,
}

impl BotWebServerState {
    pub fn new() -> Self {
        Self {
            server_handle: Mutex::new(None),
        }
    }
}

impl Default for BotWebServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// The embedded HTML dashboard served at `GET /`.
const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta http-equiv="refresh" content="5">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Claude Bots Dashboard</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: 'Courier New', monospace;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 24px;
    min-height: 100vh;
  }
  h1 {
    color: #a0c4ff;
    font-size: 22px;
    margin-bottom: 8px;
    letter-spacing: 2px;
    text-transform: uppercase;
  }
  .subtitle {
    color: #666;
    font-size: 12px;
    margin-bottom: 24px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }
  .card {
    background: #16213e;
    border: 1px solid #0f3460;
    border-radius: 8px;
    padding: 16px;
    transition: border-color .2s;
  }
  .card:hover { border-color: #a0c4ff; }
  .card-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .dot {
    width: 10px; height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.idle      { background: #888; }
  .dot.working   { background: #00ff88; animation: pulse 1.2s infinite; }
  .dot.error     { background: #ff6b6b; }
  .dot.stopped   { background: #ff9f43; }
  .dot.paused    { background: #ffe66d; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: .4; }
  }
  .card-name {
    font-size: 15px;
    font-weight: bold;
    color: #e0e0e0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-meta {
    font-size: 12px;
    color: #888;
    margin-top: 4px;
  }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: bold;
    text-transform: uppercase;
    margin-right: 6px;
  }
  .badge.trigger-obedient   { background: #0f3460; color: #a0c4ff; }
  .badge.trigger-proactive  { background: #1a3a1a; color: #00ff88; }
  .badge.trigger-scheduled  { background: #3a2a0a; color: #ffe66d; }
  .empty {
    text-align: center;
    padding: 48px;
    color: #555;
    font-size: 14px;
  }
</style>
</head>
<body>
<h1>Claude Bots</h1>
<p class="subtitle">Auto-refreshes every 5 seconds</p>
<div id="app" class="empty">Loading bots&hellip;</div>
<script>
  fetch('/api/bots')
    .then(r => r.json())
    .then(bots => {
      const app = document.getElementById('app');
      if (!bots.length) {
        app.innerHTML = '<p class="empty">No bots created yet.</p>';
        return;
      }
      app.className = 'grid';
      app.innerHTML = bots.map(b => `
        <div class="card">
          <div class="card-header">
            <span class="dot ${b.status}"></span>
            <span class="card-name">${esc(b.name)}</span>
          </div>
          <div>
            <span class="badge trigger-${b.triggerType}">${esc(b.triggerType)}</span>
            <span class="badge" style="background:#222;color:#aaa">${esc(b.status)}</span>
          </div>
          <p class="card-meta">ID: ${esc(b.id.slice(0,8))}&hellip;</p>
        </div>
      `).join('');
    })
    .catch(() => {
      document.getElementById('app').innerHTML =
        '<p class="empty">Failed to load bots.</p>';
    });

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s || '';
    return d.innerHTML;
  }
</script>
</body>
</html>"##;

/// Parse an extremely simple HTTP request line.  Returns `(method, path)`.
fn parse_request_line(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    Some((method, path))
}

/// Start the bot dashboard web server on a random port.
/// Returns the port number.
#[tauri::command]
pub fn bot_start_web_server(
    state: State<'_, BotWebServerState>,
) -> Result<u16, String> {
    let mut handle = state.server_handle.lock().unwrap();

    // Already running?
    if let Some(ref h) = *handle {
        return Ok(h.port);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to bind web server: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .port();

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = shutdown_flag.clone();

    // Set non-blocking so we can poll the shutdown flag
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

    std::thread::spawn(move || {
        log::info!("Bot web server started on port {}", port);

        while !flag_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    // Read the request (up to 4 KB is plenty for our use case)
                    let mut buf = [0u8; 4096];
                    let n = match stream.read(&mut buf) {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    let request = String::from_utf8_lossy(&buf[..n]);

                    let first_line = request.lines().next().unwrap_or("");
                    let (method, path) = match parse_request_line(first_line) {
                        Some(mp) => mp,
                        None => continue,
                    };

                    if method != "GET" {
                        let resp = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n";
                        let _ = stream.write_all(resp.as_bytes());
                        continue;
                    }

                    match path {
                        "/" => {
                            let body = DASHBOARD_HTML;
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        }
                        "/api/bots" => {
                            let body = match read_index() {
                                Ok(bots) => serde_json::to_string(&bots).unwrap_or_else(|_| "[]".to_string()),
                                Err(_) => "[]".to_string(),
                            };
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        }
                        p if p.starts_with("/api/bots/") => {
                            let bot_id = &p["/api/bots/".len()..];
                            let (status, body) = match read_bot_config(bot_id) {
                                Ok(bot) => (
                                    "200 OK",
                                    serde_json::to_string(&bot).unwrap_or_else(|_| "{}".to_string()),
                                ),
                                Err(e) => (
                                    "404 Not Found",
                                    format!("{{\"error\":\"{}\"}}", e.replace('"', "\\\"")),
                                ),
                            };
                            let resp = format!(
                                "HTTP/1.1 {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                status,
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        }
                        _ => {
                            let body = r#"{"error":"Not Found"}"#;
                            let resp = format!(
                                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No incoming connection — sleep briefly to avoid busy loop
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    log::warn!("Bot web server accept error: {}", e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        log::info!("Bot web server on port {} shut down", port);
    });

    *handle = Some(BotWebServerHandle {
        port,
        shutdown_flag,
    });

    Ok(port)
}

/// Stop the bot dashboard web server.
#[tauri::command]
pub fn bot_stop_web_server(
    state: State<'_, BotWebServerState>,
) -> Result<(), String> {
    let mut handle = state.server_handle.lock().unwrap();
    if let Some(h) = handle.take() {
        h.shutdown_flag.store(true, Ordering::Relaxed);
        log::info!("Bot web server shutdown requested (port {})", h.port);
    }
    Ok(())
}

/// Return the port the server is listening on, or `null` if not running.
#[tauri::command]
pub fn bot_get_web_server_status(
    state: State<'_, BotWebServerState>,
) -> Result<Option<u16>, String> {
    let handle = state.server_handle.lock().unwrap();
    Ok(handle.as_ref().map(|h| h.port))
}

// ============================================================================
// Tauri command — Message filter (pre-filter for channel messages via Agent SDK)
// ============================================================================

/// Quick message filter using the Claude Agent SDK (same infrastructure as the
/// coworks sidecar).  Spawns `filter.js` from the bundled coworks-sidecar resources
/// as a one-shot Node.js process, passes the request via stdin, and reads the
/// `{ "pass": bool }` result from stdout.
///
/// `env` should contain at minimum `ANTHROPIC_AUTH_TOKEN` (and optionally
/// `ANTHROPIC_BASE_URL` / `ANTHROPIC_MODEL`) matching the bot's provider config.
///
/// Fails open: any spawn/timeout/parse failure returns `Ok(true)`.
#[tauri::command]
pub async fn bot_filter_message(
    app_handle: tauri::AppHandle,
    content: String,
    system_prompt: String,
    model: String,
    env: Option<std::collections::HashMap<String, String>>,
) -> Result<bool, String> {
    let node_path = crate::nodejs::get_nodejs_path()
        .ok_or_else(|| "Node.js not found".to_string())?;

    let filter_script = get_filter_script_path(&app_handle)?;

    // Build PATH with npm global bin (Agent SDK needs the `claude` CLI in PATH)
    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut path_parts: Vec<String> = vec![];
    if let Some(npm_bin) = crate::nodejs::get_npm_global_bin_path() {
        path_parts.push(npm_bin);
    }
    if let Some(node_dir) = std::path::Path::new(&node_path).parent() {
        path_parts.push(node_dir.to_string_lossy().into_owned());
    }
    path_parts.push(current_path);
    let new_path = path_parts.join(if cfg!(windows) { ";" } else { ":" });

    let sidecar_dir = std::path::Path::new(&filter_script)
        .parent()
        .ok_or_else(|| "Failed to get filter script dir".to_string())?
        .to_path_buf();

    let mut cmd = tokio::process::Command::new(&node_path);
    cmd.arg(&filter_script)
        .current_dir(&sidecar_dir)
        .env("PATH", &new_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    if let Some(ref vars) = env {
        cmd.envs(vars);
    }

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("bot_filter_message: spawn failed: {}", e))?;
    log::info!("bot_filter_message: spawned filter.js (pid={:?})", child.id());

    // Take stdout before writing stdin — must happen before wait_with_output-style calls.
    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| "bot_filter_message: no stdout handle".to_string())?;

    // Send request via stdin
    let request = serde_json::json!({
        "content": content,
        "systemPrompt": system_prompt,
        "model": model,
    });
    if let Some(mut stdin) = child.stdin.take() {
        log::info!("bot_filter_message: writing stdin");
        let _ = tokio::io::AsyncWriteExt::write_all(
            &mut stdin,
            (request.to_string() + "\n").as_bytes(),
        )
        .await;
        log::info!("bot_filter_message: stdin written, dropping to signal EOF");
        // Drop stdin to signal EOF so the child script knows no more input is coming.
    }

    log::info!("bot_filter_message: waiting for first line of stdout (60s timeout)");
    // Read ONE line from stdout, then kill the child.
    // We must NOT use read_to_end: Agent SDK internally spawns cli.js which inherits
    // filter.js's stdout fd.  Even after filter.js calls process.exit(0), cli.js keeps
    // the write-end of the pipe open, so read_to_end never sees EOF and tokio's timeout
    // cannot interrupt a blocking OS read.  Reading a single line avoids this entirely.
    let reader = tokio::io::BufReader::new(child_stdout);
    let mut lines_reader = {
        use tokio::io::AsyncBufReadExt;
        reader.lines()
    };
    let line = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        lines_reader.next_line(),
    )
    .await
    {
        Ok(Ok(Some(l))) => {
            // Kill child (and inherited cli.js) now that we have the result.
            let _ = child.kill().await;
            let _ = child.wait().await;
            l
        }
        Ok(Ok(None)) => {
            // EOF without output — fail-open.
            log::warn!("bot_filter_message: stdout closed without output — failing open");
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Ok(true);
        }
        Ok(Err(e)) => {
            log::warn!("bot_filter_message: read failed: {} — killing child, failing open", e);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Ok(true);
        }
        Err(_) => {
            log::warn!("bot_filter_message: 60s timeout — killing child, failing open");
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Ok(true);
        }
    };

    // Parse the single line result: { "pass": bool }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
        let answer = val.get("answer").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(pass) = val.get("pass").and_then(|v| v.as_bool()) {
            log::info!("bot_filter_message: result pass={} answer={:?}", pass, answer);
            return Ok(pass);
        }
    }

    log::warn!("bot_filter_message: could not parse result {:?} — failing open", line);
    Ok(true) // fail-open if no valid response line
}

/// Locate `filter.js` in the bundled coworks-sidecar resources.
fn get_filter_script_path<R: tauri::Runtime>(
    _app_handle: &tauri::AppHandle<R>,
) -> Result<String, String> {
    // Production: bundled alongside the coworks-sidecar JS files
    if let Some(resources_dir) = crate::nodejs::get_bundled_resources_dir() {
        let p = resources_dir.join("coworks-sidecar").join("filter.js");
        if p.exists() {
            return Ok(p.to_string_lossy().into_owned());
        }
    }
    // Development fallback
    let dev = std::path::PathBuf::from("resources")
        .join("coworks-sidecar")
        .join("filter.js");
    if dev.exists() {
        return Ok(dev.to_string_lossy().into_owned());
    }
    Err("filter.js not found in coworks-sidecar resources".to_string())
}
