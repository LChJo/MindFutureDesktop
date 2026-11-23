//! Feishu (Lark) channel implementation.
//!
//! - Auth: tenant_access_token (app_id + app_secret), cached with 2h expiry
//! - Outbound: POST /im/v1/messages (REST API)
//! - Inbound: WebSocket long connection via Node.js SDK (event subscription)

use super::{Channel, ChannelAttachment, ChannelMessage, SendMessage};
use super::{CONN_STATE_CONNECTED, CONN_STATE_CONNECTING, CONN_STATE_DISCONNECTED};
use crate::nodejs::{get_bundled_resources_dir, get_nodejs_path};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const API_BASE: &str = "https://open.feishu.cn/open-apis";

/// Persist Feishu dedup state for one day to survive listener/app restarts.
const DEDUP_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Hard upper bound for persisted dedup entries after pruning.
const DEDUP_MAX_ENTRIES: usize = 5000;

/// Seconds to wait before restarting a crashed listener process
const RESTART_DELAY_SECS: u64 = 5;

pub struct FeishuChannel {
    bot_id: String,
    app_id: String,
    app_secret: String,
    chat_id: String,
    reply_all_group_messages: bool,
    client: reqwest::blocking::Client,
    /// Cached token + expiry instant
    token_cache: Arc<Mutex<Option<(String, Instant)>>>,
}

impl FeishuChannel {
    pub fn new(
        bot_id: String,
        app_id: String,
        app_secret: String,
        chat_id: String,
        reply_all_group_messages: bool,
    ) -> Self {
        // Get shared token cache BEFORE moving app_id into the struct.
        let token_cache = get_reaction_token_cache(&app_id);
        Self {
            bot_id,
            app_id,
            app_secret,
            chat_id,
            reply_all_group_messages,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            token_cache,
        }
    }

    /// Get or refresh the tenant_access_token.
    fn get_token(&self) -> Result<String, String> {
        fetch_token(
            &self.client,
            &self.token_cache,
            &self.app_id,
            &self.app_secret,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistedDedupState {
    #[serde(default)]
    message_ids: HashMap<String, u64>,
}

struct PersistentDedupSet {
    path: PathBuf,
    entries: HashMap<String, u64>,
}

#[derive(Debug)]
enum DedupInsertOutcome {
    Accepted,
    Duplicate { age_ms: u64 },
}

impl PersistentDedupSet {
    fn load(bot_id: &str) -> Self {
        Self::load_from_path(feishu_dedup_path(bot_id))
    }

    fn load_from_path(path: PathBuf) -> Self {
        let mut state = Self {
            path,
            entries: HashMap::new(),
        };

        if state.path.exists() {
            match std::fs::read_to_string(&state.path) {
                Ok(raw) => match serde_json::from_str::<PersistedDedupState>(&raw) {
                    Ok(saved) => state.entries = saved.message_ids,
                    Err(e) => log::warn!(
                        "Feishu dedup: failed to parse state {:?}: {}",
                        state.path,
                        e
                    ),
                },
                Err(e) => log::warn!("Feishu dedup: failed to read state {:?}: {}", state.path, e),
            }
        }

        let now_ms = now_millis();
        let original_count = state.entries.len();
        let pruned = state.prune(now_ms);
        log::info!(
            "Feishu dedup dedup_state_loaded: path={} entries={} pruned={} ttl_ms={} at_local={} at_ms={}",
            state.path.display(),
            original_count,
            pruned,
            DEDUP_TTL_MS,
            local_timestamp_string(),
            now_ms
        );
        if pruned > 0 {
            match state.persist() {
                Ok(()) => {
                    log::info!(
                        "Feishu dedup dedup_state_pruned: path={} removed={} remaining={} reason=load at_local={} at_ms={}",
                        state.path.display(),
                        pruned,
                        state.entries.len(),
                        local_timestamp_string(),
                        now_ms
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Feishu dedup dedup_state_prune_persist_failed: path={} removed={} remaining={} error={} reason=load at_local={} at_ms={}",
                        state.path.display(),
                        pruned,
                        state.entries.len(),
                        e,
                        local_timestamp_string(),
                        now_ms
                    );
                }
            }
        }
        state
    }

    fn insert(&mut self, message_id: &str, now_ms: u64) -> Result<DedupInsertOutcome, String> {
        let pruned = self.prune(now_ms);
        if pruned > 0 {
            self.persist()?;
            log::info!(
                "Feishu dedup dedup_state_pruned: path={} removed={} remaining={} reason=insert at_local={} at_ms={}",
                self.path.display(),
                pruned,
                self.entries.len(),
                local_timestamp_string(),
                now_ms
            );
        }

        if self.entries.contains_key(message_id) {
            let age_ms = self.existing_age_ms(message_id, now_ms).unwrap_or(0);
            return Ok(DedupInsertOutcome::Duplicate { age_ms });
        }

        self.entries.insert(message_id.to_string(), now_ms);
        if let Err(e) = self.persist() {
            self.entries.remove(message_id);
            return Err(e);
        }

        Ok(DedupInsertOutcome::Accepted)
    }

    fn existing_age_ms(&self, message_id: &str, now_ms: u64) -> Option<u64> {
        self.entries
            .get(message_id)
            .map(|seen_ms| now_ms.saturating_sub(*seen_ms))
    }

    fn prune(&mut self, now_ms: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, seen_ms| now_ms.saturating_sub(*seen_ms) <= DEDUP_TTL_MS);

        if self.entries.len() > DEDUP_MAX_ENTRIES {
            let mut by_newest: Vec<(String, u64)> = self
                .entries
                .iter()
                .map(|(id, seen_ms)| (id.clone(), *seen_ms))
                .collect();
            by_newest.sort_by(|a, b| b.1.cmp(&a.1));
            by_newest.truncate(DEDUP_MAX_ENTRIES);
            self.entries = by_newest.into_iter().collect();
        }

        before.saturating_sub(self.entries.len())
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }

        let payload = PersistedDedupState {
            message_ids: self.entries.clone(),
        };

        let content = serde_json::to_string(&payload)
            .map_err(|e| format!("failed to serialize {}: {}", self.path.display(), e))?;
        std::fs::write(&self.path, content)
            .map_err(|e| format!("failed to write {}: {}", self.path.display(), e))
    }
}

/// Fetch or refresh the tenant_access_token.
/// Standalone function so it can be shared between FeishuChannel methods and listener threads.
fn fetch_token(
    client: &reqwest::blocking::Client,
    token_cache: &Arc<Mutex<Option<(String, Instant)>>>,
    app_id: &str,
    app_secret: &str,
) -> Result<String, String> {
    let mut cache = token_cache.lock().unwrap();

    if let Some((ref token, ref expiry)) = *cache {
        if Instant::now() < *expiry {
            return Ok(token.clone());
        }
    }

    let url = format!("{}/auth/v3/tenant_access_token/internal/", API_BASE);
    let body = serde_json::json!({
        "app_id": app_id,
        "app_secret": app_secret,
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("Feishu auth request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Feishu auth parse failed: {}", e))?;

    let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = data
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("Feishu auth error (code {}): {}", code, msg));
    }

    let token = data
        .get("tenant_access_token")
        .and_then(|t| t.as_str())
        .ok_or("Missing tenant_access_token in response")?
        .to_string();

    let expire_secs = data.get("expire").and_then(|e| e.as_u64()).unwrap_or(7200);

    let expiry = Instant::now() + std::time::Duration::from_secs(expire_secs.saturating_sub(600));
    *cache = Some((token.clone(), expiry));

    Ok(token)
}

/// Add the `Status_PrivateMessage` acknowledgment reaction to a message.
/// Best-effort: failures are logged but do not interrupt message forwarding.
fn add_reaction(client: &reqwest::blocking::Client, token: &str, message_id: &str) {
    let url = format!("{}/im/v1/messages/{}/reactions", API_BASE, message_id);
    let body = serde_json::json!({
        "reaction_type": {
            "emoji_type": "Status_PrivateMessage"
        }
    });

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>() {
                let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                if code != 0 {
                    let msg = data
                        .get("msg")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown");
                    log::warn!("Feishu add_reaction failed (code {}): {}", code, msg);
                }
            }
        }
        Err(e) => {
            log::warn!("Feishu add_reaction request failed: {}", e);
        }
    }
}

/// Download a resource (image/file) from Feishu API to local disk.
/// Saves to: ~/.MoontonClaudeDesktop/cache/feishu_downloads/{message_id}/{file_name}
fn download_resource(
    client: &reqwest::blocking::Client,
    token: &str,
    message_id: &str,
    file_key: &str,
    resource_type: &str,
    file_name: &str,
) -> Result<(String, String), String> {
    let save_dir = dirs::home_dir()
        .ok_or_else(|| "No home directory".to_string())?
        .join(".MoontonClaudeDesktop")
        .join("cache")
        .join("feishu_downloads")
        .join(message_id);

    std::fs::create_dir_all(&save_dir)
        .map_err(|e| format!("Failed to create download dir: {}", e))?;

    let url = format!(
        "{}/im/v1/messages/{}/resources/{}?type={}",
        API_BASE, message_id, file_key, resource_type
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download HTTP error: {}", resp.status()));
    }

    // Sanitize file_name to prevent path traversal attacks
    let safe_name: String = file_name
        .replace(['/', '\\'], "_")
        .replace("..", "_")
        .trim_start_matches('_')
        .to_string();
    let safe_name = if safe_name.is_empty() {
        "file".to_string()
    } else {
        safe_name
    };
    let save_path = save_dir.join(&safe_name);
    let bytes = resp
        .bytes()
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    std::fs::write(&save_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!(
        "Downloaded Feishu resource: {} -> {:?} ({} bytes)",
        file_key,
        save_path,
        bytes.len()
    );

    Ok((save_path.to_string_lossy().to_string(), safe_name))
}

// ---------------------------------------------------------------------------
// Outbound: Markdown → Feishu interactive card
// ---------------------------------------------------------------------------

/// Max characters per markdown element in Feishu cards.
const CARD_MD_CHUNK_SIZE: usize = 4000;

/// Max total card JSON size (bytes).
const CARD_MAX_SIZE: usize = 28000;

/// JSON overhead per card element: {"tag":"markdown","content":""}
const ELEMENT_OVERHEAD: usize = 32;

/// Fixed card skeleton overhead: {"config":{...},"elements":[...]}
const CARD_SKELETON_OVERHEAD: usize = 60;

const TRUNCATION_MARKER: &str = "...\n\n*[消息过长，已截断]*";

/// Build a Feishu interactive card as a JSON string.
/// Feishu API expects `content` to be a stringified JSON value.
fn build_card_content(text: &str) -> String {
    let adapted = adapt_markdown_for_feishu(text);
    let chunks = split_markdown_chunks(&adapted, CARD_MD_CHUNK_SIZE);

    let elements: Vec<serde_json::Value> = chunks
        .into_iter()
        .map(|chunk| serde_json::json!({ "tag": "markdown", "content": chunk }))
        .collect();

    let card = serde_json::json!({
        "config": { "wide_screen_mode": true },
        "elements": elements,
    });

    card.to_string()
}

/// Extract heading text from a Markdown heading line (e.g. "## Foo" → "Foo").
/// Returns None if the line is not a heading.
fn extract_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    // Find the byte offset of the first non-'#' character
    let byte_offset = trimmed
        .char_indices()
        .find(|(_, c)| *c != '#')
        .map(|(i, _)| i)
        .unwrap_or(trimmed.len());
    let hash_count = byte_offset; // '#' is ASCII, so byte offset == char count
    if hash_count > 6 {
        return None;
    }
    let rest = &trimmed[byte_offset..];
    // Must be followed by space or be empty
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim())
}

/// Check if a line is a code fence (opening or closing).
/// Opening: ```lang  or  ```
/// Closing: ```
fn is_code_fence(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with("```") {
        return false;
    }
    let after = t[3..].trim();
    // Closing fence: empty after backticks
    // Opening fence: only a language identifier (alphanumeric, -, _)
    after.is_empty()
        || (!after.contains('`')
            && after
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' '))
}

/// Adapt standard Markdown to Feishu card-compatible markdown.
/// - Converts headings to bold (Feishu card markdown doesn't render # headings)
/// - Preserves code blocks untouched
fn adapt_markdown_for_feishu(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_code_block = false;

    for line in text.lines() {
        if is_code_fence(line) {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Convert headings to bold (# Heading → **Heading**)
        if let Some(heading_text) = extract_heading(line) {
            if !heading_text.is_empty() {
                result.push_str("**");
                result.push_str(heading_text);
                result.push_str("**");
            }
            result.push('\n');
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Split markdown text into chunks at paragraph boundaries (double newlines),
/// each chunk staying within `max_chunk_size` characters.
/// Respects code block boundaries — never splits inside a code fence.
/// Truncates with a note if total card size would exceed CARD_MAX_SIZE.
fn split_markdown_chunks(text: &str, max_chunk_size: usize) -> Vec<String> {
    if text.len() <= max_chunk_size {
        return vec![text.to_string()];
    }

    // Split into segments at \n\n boundaries, but merge segments that are
    // inside a code block back together to avoid breaking code fences.
    let raw_paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut paragraphs: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut merged = String::new();

    for para in &raw_paragraphs {
        let fence_count = para.lines().filter(|l| is_code_fence(l)).count();

        if in_code_block {
            // Inside a code block — merge this paragraph back
            merged.push_str("\n\n");
            merged.push_str(para);
            // Toggle for each fence found
            for _ in 0..fence_count {
                in_code_block = !in_code_block;
            }
            if !in_code_block {
                // Code block closed — flush merged content
                paragraphs.push(std::mem::take(&mut merged));
            }
        } else {
            // Not in a code block
            if fence_count % 2 == 1 {
                // Odd fences: code block opens and doesn't close in this paragraph
                if !merged.is_empty() {
                    paragraphs.push(std::mem::take(&mut merged));
                }
                merged = para.to_string();
                in_code_block = true;
            } else {
                // Even fences (0 or balanced): standalone paragraph
                paragraphs.push(para.to_string());
            }
        }
    }
    // Flush any remaining merged content
    if !merged.is_empty() {
        paragraphs.push(merged);
    }

    // Group paragraphs into chunks respecting max_chunk_size
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    for para in &paragraphs {
        let addition = if current.is_empty() {
            para.clone()
        } else {
            format!("\n\n{}", para)
        };

        if current.len() + addition.len() > max_chunk_size && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            current = para.clone();
        } else {
            current.push_str(&addition);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    // Hard-split any individual chunk that still exceeds max_chunk_size
    // (e.g. a single huge code block or wall of text)
    let mut split_chunks: Vec<String> = Vec::new();
    for chunk in chunks {
        if chunk.len() <= max_chunk_size {
            split_chunks.push(chunk);
        } else {
            // Split at line boundaries
            let mut sub = String::new();
            for line in chunk.lines() {
                let line_add = if sub.is_empty() {
                    line.len()
                } else {
                    line.len() + 1 // +1 for '\n'
                };
                if sub.len() + line_add > max_chunk_size && !sub.is_empty() {
                    split_chunks.push(std::mem::take(&mut sub));
                }
                if !sub.is_empty() {
                    sub.push('\n');
                }
                sub.push_str(line);
            }
            if !sub.is_empty() {
                split_chunks.push(sub);
            }
        }
    }

    // Truncate if total card size would be too large
    let marker_cost = TRUNCATION_MARKER.len() + ELEMENT_OVERHEAD;
    let mut total: usize = CARD_SKELETON_OVERHEAD;
    let mut final_chunks: Vec<String> = Vec::new();

    for chunk in split_chunks {
        let cost = chunk.len() + ELEMENT_OVERHEAD;
        if total + cost + marker_cost > CARD_MAX_SIZE && !final_chunks.is_empty() {
            final_chunks.push(TRUNCATION_MARKER.to_string());
            break;
        }
        total += cost;
        final_chunks.push(chunk);
    }

    if final_chunks.is_empty() {
        final_chunks.push(text[..max_chunk_size.min(text.len())].to_string());
    }

    final_chunks
}

/// Locate the event-listener.cjs script.
///
/// Priority:
/// 1. Bundled resources (production)
/// 2. Relative path from exe working directory (development)
fn find_event_listener_script() -> Result<PathBuf, String> {
    // Production: bundled resources
    if let Some(resources_dir) = get_bundled_resources_dir() {
        let prod_path = resources_dir.join("feishu").join("event-listener.cjs");
        if prod_path.exists() {
            log::info!(
                "Found Feishu event listener at bundled resources: {:?}",
                prod_path
            );
            return Ok(prod_path);
        }
    }

    // Development: relative to CWD (src-tauri/)
    let dev_path = PathBuf::from("resources")
        .join("feishu")
        .join("event-listener.cjs");
    if dev_path.exists() {
        log::info!("Found Feishu event listener at dev path: {:?}", dev_path);
        return Ok(dev_path);
    }

    Err("Feishu event-listener.cjs not found".to_string())
}

fn feishu_dedup_path(bot_id: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".MoontonClaudeDesktop")
        .join("bots")
        .join(bot_id)
        .join("feishu_dedup.json")
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn local_timestamp_string() -> String {
    Local::now().to_rfc3339()
}

impl Channel for FeishuChannel {
    fn name(&self) -> &str {
        "feishu"
    }

    fn validate_config(&self) -> Result<(), String> {
        if self.app_id.is_empty() {
            return Err("App ID is required".to_string());
        }
        if self.app_secret.is_empty() {
            return Err("App Secret is required".to_string());
        }
        // chat_id is optional — only needed for proactive messages;
        // replies use the dynamic target_chat_id from incoming messages.
        Ok(())
    }

    fn send(&self, msg: &SendMessage) -> Result<(), String> {
        let receive_id = msg
            .target_chat_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.chat_id);

        if receive_id.is_empty() {
            return Err(
                "No target chat_id: configure a default Chat ID or reply to an incoming message"
                    .to_string(),
            );
        }

        let token = self.get_token()?;

        let url = format!("{}/im/v1/messages?receive_id_type=chat_id", API_BASE);

        let content = build_card_content(&msg.content);
        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": "interactive",
            "content": content,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .map_err(|e| format!("Feishu send failed: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .map_err(|e| format!("Feishu response parse failed: {}", e))?;

        let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = data
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(format!("Feishu send error (code {}): {}", code, msg));
        }

        let outbound_message_id = data
            .pointer("/data/message_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                data.pointer("/data/items/0/message_id")
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("");
        let logged_at_ms = now_millis();

        log::info!(
            "Feishu message sent: bot={} target_chat={} chars={} outbound_msg_id={} at_local={} at_ms={}",
            self.bot_id,
            receive_id,
            msg.content.len(),
            outbound_message_id,
            local_timestamp_string(),
            logged_at_ms
        );
        Ok(())
    }

    fn start_listener(
        &self,
        tx: Sender<ChannelMessage>,
        shutdown: Arc<AtomicBool>,
        conn_state: Arc<AtomicU8>,
    ) -> Result<(), String> {
        let bot_id = self.bot_id.clone();
        let app_id = self.app_id.clone();
        let app_secret = self.app_secret.clone();
        let reply_all = self.reply_all_group_messages;
        let ack_client = self.client.clone();
        let ack_token_cache = self.token_cache.clone();

        // Pre-validate paths before spawning the thread
        let node_path = get_nodejs_path()
            .ok_or("Node.js not found. Please install Node.js to use Feishu channel.")?;
        let script_path = find_event_listener_script()?;

        std::thread::spawn(move || {
            log::info!(
                "Feishu WebSocket listener thread started for bot {}",
                bot_id
            );

            let mut dedup = PersistentDedupSet::load(&bot_id);

            while !shutdown.load(Ordering::Relaxed) {
                conn_state.store(CONN_STATE_CONNECTING, Ordering::Relaxed);
                log::info!(
                    "Spawning Feishu event-listener process for bot {}: {:?} {:?}",
                    bot_id,
                    node_path,
                    script_path
                );

                let mut cmd = Command::new(&node_path);
                cmd.arg(&script_path)
                    .arg(&app_id)
                    .arg(&app_secret)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .stdin(Stdio::null());

                #[cfg(target_os = "windows")]
                cmd.creation_flags(CREATE_NO_WINDOW);

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to spawn Feishu listener process: {}", e);
                        // Wait before retry
                        sleep_with_shutdown(&shutdown, RESTART_DELAY_SECS);
                        continue;
                    }
                };

                // Read stdout line by line
                if let Some(stdout) = child.stdout.take() {
                    let reader = std::io::BufReader::new(stdout);
                    for line in reader.lines() {
                        if shutdown.load(Ordering::Relaxed) {
                            break;
                        }

                        let line = match line {
                            Ok(l) => l,
                            Err(e) => {
                                log::warn!("Feishu listener stdout read error: {}", e);
                                break;
                            }
                        };

                        if line.trim().is_empty() {
                            continue;
                        }

                        let parsed: serde_json::Value = match serde_json::from_str(&line) {
                            Ok(v) => v,
                            Err(e) => {
                                log::warn!("Feishu listener: invalid JSON line: {} ({})", line, e);
                                continue;
                            }
                        };

                        // Handle lifecycle signals
                        if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                            match msg_type {
                                "ready" => {
                                    conn_state.store(CONN_STATE_CONNECTED, Ordering::Relaxed);
                                    log::info!(
                                        "Feishu WebSocket connection established for bot {}",
                                        bot_id
                                    );
                                    continue;
                                }
                                "error" => {
                                    let err_msg = parsed
                                        .get("msg")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("unknown");
                                    log::error!("Feishu listener error: {}", err_msg);
                                    continue;
                                }
                                "debug" => {
                                    log::info!("Feishu listener debug: {}", parsed);
                                    continue;
                                }
                                "sdk_log" => {
                                    let level = parsed
                                        .get("level")
                                        .and_then(|l| l.as_str())
                                        .unwrap_or("info");
                                    let args = parsed
                                        .get("args")
                                        .and_then(|a| a.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(|v| v.as_str())
                                                .collect::<Vec<_>>()
                                                .join(" ")
                                        })
                                        .unwrap_or_default();
                                    match level {
                                        "error" => log::error!("Feishu SDK: {}", args),
                                        "warn" => log::warn!("Feishu SDK: {}", args),
                                        _ => log::info!("Feishu SDK: {}", args),
                                    }
                                    continue;
                                }
                                _ => {}
                            }
                        }

                        // Parse message event
                        let message_id = parsed
                            .get("message_id")
                            .and_then(|m| m.as_str())
                            .unwrap_or("")
                            .to_string();

                        let text = parsed
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        let sender_id = parsed
                            .get("sender_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let chat_id = parsed
                            .get("chat_id")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        let mentioned_bot = parsed
                            .get("mentioned_bot")
                            .and_then(|m| m.as_bool())
                            .unwrap_or(false);
                        let event_source = parsed
                            .get("event_source")
                            .and_then(|s| s.as_str())
                            .unwrap_or("message");
                        let attachment_count = parsed
                            .get("resources")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.len())
                            .unwrap_or(0);

                        // Group message filter: when reply_all is false,
                        // only process @bot mentions in groups.
                        let chat_type = parsed
                            .get("chat_type")
                            .and_then(|c| c.as_str())
                            .unwrap_or("");
                        if chat_type == "group" && !reply_all {
                            if !mentioned_bot {
                                log::info!(
                                    "Feishu inbound skipped: bot={} source={} reason=non_mention chat_id={} chat_type={} msg_id={} sender={} mentioned_bot={} at_local={} at_ms={}",
                                    bot_id,
                                    event_source,
                                    chat_id,
                                    chat_type,
                                    message_id,
                                    sender_id,
                                    mentioned_bot,
                                    local_timestamp_string(),
                                    now_millis()
                                );
                                continue;
                            }
                        }

                        // Dedup by message_id after mention filtering so a later
                        // explicit reaction-triggered replay of the same group message
                        // can still be processed even if the original non-mention
                        // event was skipped above.
                        let inbound_logged_at_ms = now_millis();
                        if !message_id.is_empty() {
                            match dedup.insert(&message_id, inbound_logged_at_ms) {
                                Ok(DedupInsertOutcome::Accepted) => {
                                    log::info!(
                                        "Feishu dedup dedup_accept_persisted: bot={} source={} msg_id={} chat_id={} sender={} path={} entries={} at_local={} at_ms={}",
                                        bot_id,
                                        event_source,
                                        message_id,
                                        chat_id,
                                        sender_id,
                                        dedup.path.display(),
                                        dedup.entries.len(),
                                        local_timestamp_string(),
                                        inbound_logged_at_ms
                                    );
                                }
                                Ok(DedupInsertOutcome::Duplicate { age_ms }) => {
                                    log::info!(
                                        "Feishu dedup dedup_duplicate_blocked: bot={} source={} msg_id={} chat_id={} sender={} age_ms={} path={} at_local={} at_ms={}",
                                        bot_id,
                                        event_source,
                                        message_id,
                                        chat_id,
                                        sender_id,
                                        age_ms,
                                        dedup.path.display(),
                                        local_timestamp_string(),
                                        inbound_logged_at_ms
                                    );
                                    continue;
                                }
                                Err(e) => {
                                    log::error!(
                                        "Feishu dedup dedup_accept_persist_failed: bot={} source={} msg_id={} chat_id={} sender={} error={} path={} at_local={} at_ms={}",
                                        bot_id,
                                        event_source,
                                        message_id,
                                        chat_id,
                                        sender_id,
                                        e,
                                        dedup.path.display(),
                                        local_timestamp_string(),
                                        inbound_logged_at_ms
                                    );
                                    continue;
                                }
                            }
                        }

                        // Collect resource descriptors for async download
                        let resources: Vec<(String, String, String)> = parsed
                            .get("resources")
                            .and_then(|r| r.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|res| {
                                        let key = res
                                            .get("key")
                                            .and_then(|k| k.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if key.is_empty() {
                                            return None;
                                        }
                                        let res_type = res
                                            .get("type")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("file")
                                            .to_string();
                                        let name = res
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("file")
                                            .to_string();
                                        Some((key, res_type, name))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        // Skip messages with no text and no resources
                        if text.is_empty() && resources.is_empty() {
                            continue;
                        }

                        let timestamp = inbound_logged_at_ms;
                        log::info!(
                            "Feishu inbound accepted: bot={} source={} msg_id={} chat_id={} chat_type={} sender={} mentioned_bot={} attachments={} text_chars={} at_local={} at_ms={}",
                            bot_id,
                            event_source,
                            message_id,
                            chat_id,
                            chat_type,
                            sender_id,
                            mentioned_bot,
                            attachment_count,
                            text.len(),
                            local_timestamp_string(),
                            timestamp
                        );

                        // Add the acknowledgment reaction before forwarding the message to the frontend.
                        // This keeps the ack lifecycle deterministic so later clears cannot race ahead of adds.
                        if !message_id.is_empty() {
                            match fetch_token(&ack_client, &ack_token_cache, &app_id, &app_secret) {
                                Ok(token) => add_reaction(&ack_client, &token, &message_id),
                                Err(e) => log::warn!("Feishu reaction: token fetch failed: {}", e),
                            }
                        }

                        if resources.is_empty() {
                            // No resources — send immediately, no blocking
                            let channel_msg = ChannelMessage {
                                platform: "feishu".to_string(),
                                channel_id: chat_id.clone(),
                                sender: sender_id.clone(),
                                content: text.clone(),
                                timestamp,
                                attachments: Vec::new(),
                                message_id: message_id.clone(),
                            };
                            log::info!(
                                "Feishu listener forwarding inbound message: bot={} msg_id={} chat_id={} source={} attachments=0",
                                bot_id,
                                message_id,
                                chat_id,
                                event_source
                            );
                            if tx.send(channel_msg).is_err() {
                                log::info!("Feishu listener: receiver dropped, exiting");
                                let _ = kill_child(&mut child);
                                return;
                            }
                        } else {
                            // Has resources — download in a separate thread to avoid blocking the listener
                            let tx_clone = tx.clone();
                            let dl_client = ack_client.clone();
                            let dl_token_cache = ack_token_cache.clone();
                            let dl_app_id = app_id.clone();
                            let dl_app_secret = app_secret.clone();
                            let dl_bot_id = bot_id.clone();
                            let dl_message_id = message_id.clone();
                            let dl_chat_id = chat_id.clone();
                            let dl_sender_id = sender_id.clone();
                            let dl_text = text.clone();
                            let dl_event_source = event_source.to_string();
                            std::thread::spawn(move || {
                                let mut attachments: Vec<ChannelAttachment> = Vec::new();
                                if let Ok(token) = fetch_token(
                                    &dl_client,
                                    &dl_token_cache,
                                    &dl_app_id,
                                    &dl_app_secret,
                                ) {
                                    for (key, res_type, name) in &resources {
                                        match download_resource(
                                            &dl_client,
                                            &token,
                                            &dl_message_id,
                                            key,
                                            res_type,
                                            name,
                                        ) {
                                            Ok((path, dl_name)) => {
                                                attachments.push(ChannelAttachment {
                                                    path,
                                                    name: dl_name,
                                                });
                                            }
                                            Err(e) => {
                                                log::warn!(
                                                    "Failed to download Feishu resource {}: {}",
                                                    key,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                                let channel_msg = ChannelMessage {
                                    platform: "feishu".to_string(),
                                    channel_id: dl_chat_id.clone(),
                                    sender: dl_sender_id,
                                    content: dl_text,
                                    timestamp,
                                    attachments,
                                    message_id: dl_message_id,
                                };
                                log::info!(
                                    "Feishu listener forwarding inbound message: bot={} msg_id={} chat_id={} source={} attachments={}",
                                    dl_bot_id,
                                    channel_msg.message_id,
                                    dl_chat_id,
                                    dl_event_source,
                                    channel_msg.attachments.len()
                                );
                                if tx_clone.send(channel_msg).is_err() {
                                    log::info!(
                                        "Feishu listener: receiver dropped (async download thread)"
                                    );
                                }
                            });
                        }
                    }
                }

                // Collect stderr for diagnostics
                if let Some(stderr) = child.stderr.take() {
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines().take(20) {
                        if let Ok(l) = line {
                            log::warn!("Feishu listener stderr: {}", l);
                        }
                    }
                }

                let _ = kill_child(&mut child);
                conn_state.store(CONN_STATE_DISCONNECTED, Ordering::Relaxed);

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                log::warn!(
                    "Feishu listener process exited, restarting in {} seconds...",
                    RESTART_DELAY_SECS
                );
                sleep_with_shutdown(&shutdown, RESTART_DELAY_SECS);
            }

            log::info!(
                "Feishu WebSocket listener thread stopped for bot {}",
                bot_id
            );
        });

        Ok(())
    }

    fn test_connection(&self) -> Result<(), String> {
        self.validate_config()?;
        let _token = self.get_token()?;
        Ok(())
    }
}

/// Kill a child process (best effort).
fn kill_child(child: &mut std::process::Child) -> Result<(), String> {
    match child.kill() {
        Ok(_) => {
            let _ = child.wait();
            Ok(())
        }
        Err(e) => {
            // Process may have already exited
            let _ = child.wait();
            log::debug!("kill_child: {}", e);
            Ok(())
        }
    }
}

/// Sleep in small increments, checking shutdown flag.
fn sleep_with_shutdown(shutdown: &Arc<AtomicBool>, secs: u64) {
    let ticks = secs * 10;
    for _ in 0..ticks {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// ============================================================================
// Tauri commands — Feishu reaction management
// ============================================================================

/// Global token cache keyed by app_id, shared across all reaction command calls.
/// Prevents a fresh token fetch on every `feishu_add_reaction` / `feishu_clear_app_reactions` call.
static REACTION_TOKEN_CACHES: OnceLock<
    Mutex<HashMap<String, Arc<Mutex<Option<(String, Instant)>>>>>,
> = OnceLock::new();

fn get_reaction_token_cache(app_id: &str) -> Arc<Mutex<Option<(String, Instant)>>> {
    let global = REACTION_TOKEN_CACHES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = global.lock().unwrap();
    map.entry(app_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(None)))
        .clone()
}

/// Helper: get Feishu credentials for a bot and fetch (or reuse cached) access token.
fn bot_feishu_token(bot_id: &str) -> Result<(reqwest::blocking::Client, String), String> {
    use crate::channels::config::read_channels_config;
    let cfg = read_channels_config(bot_id)?;
    let feishu = cfg
        .feishu
        .ok_or("Feishu channel not configured for this bot")?;
    if feishu.app_id.is_empty() || feishu.app_secret.is_empty() {
        return Err("Feishu App ID or App Secret is empty".to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let cache = get_reaction_token_cache(&feishu.app_id);
    let token = fetch_token(&client, &cache, &feishu.app_id, &feishu.app_secret)?;
    Ok((client, token))
}

/// Clear all reactions added by this bot app on a given message.
/// Used to silently remove the Status_PrivateMessage acknowledgment reaction (e.g. after filtering blocks a message,
/// or before switching to the StatusFlashOfInspiration "processing" reaction).
#[tauri::command]
pub fn feishu_clear_app_reactions(
    bot_id: String,
    message_id: String,
    emoji_type: Option<String>,
) -> Result<u32, String> {
    log::info!(
        "feishu_clear_app_reactions: bot={} msg={} emoji_type={}",
        bot_id,
        message_id,
        emoji_type.as_deref().unwrap_or("*")
    );
    let (client, token) = bot_feishu_token(&bot_id).map_err(|e| {
        log::warn!("feishu_clear_app_reactions: token error: {}", e);
        e
    })?;

    // 1. List reactions with page_size=50 to maximize coverage in one call
    let url = format!("{}/im/v1/messages/{}/reactions", API_BASE, message_id);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("page_size", "50")])
        .send()
        .map_err(|e| format!("Failed to list reactions: {}", e))?;
    let resp = resp
        .error_for_status()
        .map_err(|e| format!("List reactions HTTP error: {}", e))?;
    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse reaction list: {}", e))?;

    let items = data
        .get("data")
        .and_then(|d| d.get("items"))
        .and_then(|i| i.as_array());

    let mut deleted_count: u32 = 0;

    if let Some(items) = items {
        for item in items {
            let operator_type = item
                .get("operator")
                .and_then(|o| o.get("operator_type"))
                .and_then(|o| o.as_str())
                .unwrap_or("");
            if operator_type != "app" {
                continue;
            }
            let item_emoji_type = item
                .get("reaction_type")
                .and_then(|rt| rt.get("emoji_type"))
                .and_then(|et| et.as_str())
                .unwrap_or("");
            if let Some(ref expected_emoji_type) = emoji_type {
                if item_emoji_type != expected_emoji_type {
                    continue;
                }
            }
            let reaction_id = item
                .get("reaction_id")
                .and_then(|r| r.as_str())
                .unwrap_or("");
            if reaction_id.is_empty() {
                continue;
            }
            // 2. Delete each app reaction (non-fatal on failure)
            let del_url = format!(
                "{}/im/v1/messages/{}/reactions/{}",
                API_BASE, message_id, reaction_id
            );
            log::info!(
                "feishu_clear_app_reactions: deleting reaction_id={} emoji_type={}",
                reaction_id,
                if item_emoji_type.is_empty() {
                    "unknown"
                } else {
                    item_emoji_type
                }
            );
            match client
                .delete(&del_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
            {
                Ok(r) if r.status().is_success() => {
                    deleted_count += 1;
                    log::info!(
                        "feishu_clear_app_reactions: deleted reaction, status={}",
                        r.status()
                    );
                }
                Ok(r) => log::warn!(
                    "feishu_clear_app_reactions: delete HTTP {} reaction_id={}",
                    r.status(),
                    reaction_id
                ),
                Err(e) => log::warn!("feishu_clear_app_reactions: delete failed: {}", e),
            }
        }
    }

    log::info!(
        "feishu_clear_app_reactions: bot={} msg={} emoji_type={} deleted_count={}",
        bot_id,
        message_id,
        emoji_type.as_deref().unwrap_or("*"),
        deleted_count
    );

    Ok(deleted_count)
}

/// Add a reaction of the given emoji type to a message.
/// Returns the reaction_id so the caller can later remove it precisely.
#[tauri::command]
pub fn feishu_add_reaction(
    bot_id: String,
    message_id: String,
    emoji_type: String,
) -> Result<String, String> {
    let (client, token) = bot_feishu_token(&bot_id)?;
    let url = format!("{}/im/v1/messages/{}/reactions", API_BASE, message_id);
    let body = serde_json::json!({
        "reaction_type": { "emoji_type": emoji_type }
    });
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .map_err(|e| format!("feishu_add_reaction request failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("feishu_add_reaction HTTP error: {}", e))?;
    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("feishu_add_reaction parse failed: {}", e))?;
    let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = data
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        return Err(format!(
            "feishu_add_reaction error (code {}): {}",
            code, msg
        ));
    }
    let reaction_id = data
        .get("data")
        .and_then(|d| d.get("reaction_id"))
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
    Ok(reaction_id)
}

/// Remove a specific reaction by reaction_id (precise removal, e.g. for StatusFlashOfInspiration after reply).
#[tauri::command]
pub fn feishu_remove_reaction(
    bot_id: String,
    message_id: String,
    reaction_id: String,
) -> Result<(), String> {
    let (client, token) = bot_feishu_token(&bot_id)?;
    let url = format!(
        "{}/im/v1/messages/{}/reactions/{}",
        API_BASE, message_id, reaction_id
    );
    let resp = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("feishu_remove_reaction request failed: {}", e))?;
    // Non-fatal HTTP errors (e.g. 404 if reaction already removed) — just log and return Ok
    if !resp.status().is_success() {
        log::warn!(
            "feishu_remove_reaction: HTTP {} for message {} reaction {}",
            resp.status(),
            message_id,
            reaction_id
        );
        return Ok(());
    }
    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("feishu_remove_reaction parse failed: {}", e))?;
    let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = data
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        log::warn!("feishu_remove_reaction: code {}: {}", code, msg);
        // Non-fatal: reaction may have already been deleted
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DedupInsertOutcome, PersistentDedupSet, DEDUP_MAX_ENTRIES, DEDUP_TTL_MS};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("moonton-claude-feishu-tests-{}-{}", name, unique))
            .join("feishu_dedup.json")
    }

    #[test]
    fn dedup_persists_new_entry_across_reload() {
        let path = unique_temp_path("persist-reload");
        let now_ms = super::now_millis();

        let mut dedup = PersistentDedupSet::load_from_path(path.clone());
        let first = dedup.insert("om_test_1", now_ms).unwrap();
        assert!(matches!(first, DedupInsertOutcome::Accepted));

        let reloaded = PersistentDedupSet::load_from_path(path.clone());
        let duplicate_age_ms = reloaded.existing_age_ms("om_test_1", now_ms + 100).unwrap();
        assert_eq!(duplicate_age_ms, 100);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn dedup_insert_reports_duplicate_age() {
        let path = unique_temp_path("duplicate-age");
        let now_ms = 10_000;

        let mut dedup = PersistentDedupSet::load_from_path(path.clone());
        assert!(matches!(
            dedup.insert("om_test_2", now_ms).unwrap(),
            DedupInsertOutcome::Accepted
        ));
        let second = dedup.insert("om_test_2", now_ms + 250).unwrap();
        match second {
            DedupInsertOutcome::Duplicate { age_ms } => assert_eq!(age_ms, 250),
            DedupInsertOutcome::Accepted => panic!("expected duplicate outcome"),
        }

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn dedup_prunes_expired_and_caps_size() {
        let path = unique_temp_path("prune-cap");
        let expired_ms = 1_000;
        let fresh_start_ms = expired_ms + DEDUP_TTL_MS + 1;

        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();

        let mut message_ids = std::collections::HashMap::new();
        message_ids.insert("expired".to_string(), expired_ms);
        for idx in 0..=(DEDUP_MAX_ENTRIES as u64) {
            message_ids.insert(format!("fresh-{}", idx), fresh_start_ms + idx);
        }
        let raw = serde_json::json!({ "message_ids": message_ids }).to_string();
        fs::write(&path, raw).unwrap();

        let dedup = PersistentDedupSet::load_from_path(path.clone());
        assert!(dedup
            .existing_age_ms("expired", fresh_start_ms + 10)
            .is_none());
        assert!(dedup.entries.len() <= DEDUP_MAX_ENTRIES);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn dedup_insert_reverts_on_persist_failure() {
        let path = unique_temp_path("persist-fail");
        fs::create_dir_all(&path).unwrap();

        let mut dedup = PersistentDedupSet::load_from_path(path.clone());
        let err = dedup
            .insert("om_test_fail", super::now_millis())
            .unwrap_err();
        assert!(err.contains("failed to write"));
        assert!(dedup
            .existing_age_ms("om_test_fail", super::now_millis())
            .is_none());

        let _ = fs::remove_dir_all(&path);
    }
}
