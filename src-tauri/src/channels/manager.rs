//! Channel manager — Tauri commands and runtime state.
//!
//! Manages active channel listeners per bot, routes outbound messages,
//! and forwards inbound messages via Tauri events.

use super::config::{read_channels_config, write_channels_config, BotChannelsConfig};
use super::discord::DiscordChannel;
use super::feishu::FeishuChannel;
use super::telegram::TelegramChannel;
use super::{Channel, ChannelMessage, SendMessage};
use super::{CONN_STATE_CONNECTED, CONN_STATE_CONNECTING, CONN_STATE_DISCONNECTED};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

struct BotChannelHandle {
    shutdown_flag: Arc<AtomicBool>,
    platforms: Vec<String>,
    /// Per-platform connection state (keyed by platform name).
    conn_states: HashMap<String, Arc<AtomicU8>>,
}

pub struct ChannelManagerState {
    active_channels: Mutex<HashMap<String, BotChannelHandle>>,
}

impl ChannelManagerState {
    pub fn new() -> Self {
        Self {
            active_channels: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for ChannelManagerState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Status response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub platform: String,
    pub connected: bool,
    /// Real-time connection state: "disconnected" | "connecting" | "connected"
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: build Channel instances from config
// ---------------------------------------------------------------------------

fn build_channels(bot_id: &str, config: &BotChannelsConfig) -> Vec<Box<dyn Channel>> {
    let mut channels: Vec<Box<dyn Channel>> = Vec::new();

    if let Some(ref tg) = config.telegram {
        if tg.enabled {
            channels.push(Box::new(TelegramChannel::new(
                tg.bot_token.clone(),
                tg.chat_id.clone(),
            )));
        }
    }

    if let Some(ref dc) = config.discord {
        if dc.enabled {
            channels.push(Box::new(DiscordChannel::new(
                dc.bot_token.clone(),
                dc.channel_id.clone(),
            )));
        }
    }

    if let Some(ref fs) = config.feishu {
        if fs.enabled {
            channels.push(Box::new(FeishuChannel::new(
                bot_id.to_string(),
                fs.app_id.clone(),
                fs.app_secret.clone(),
                fs.chat_id.clone(),
                fs.reply_all_group_messages,
            )));
        }
    }

    channels
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Read the channel configuration for a bot.
#[tauri::command]
pub fn channel_get_config(bot_id: String) -> Result<BotChannelsConfig, String> {
    read_channels_config(&bot_id)
}

/// Save the channel configuration for a bot.
#[tauri::command]
pub fn channel_save_config(bot_id: String, config: BotChannelsConfig) -> Result<(), String> {
    write_channels_config(&bot_id, &config)
}

/// Test connection for a specific platform.
#[tauri::command]
pub fn channel_test_connection(bot_id: String, platform: String) -> Result<(), String> {
    let config = read_channels_config(&bot_id)?;

    match platform.as_str() {
        "telegram" => {
            let tg = config
                .telegram
                .ok_or("Telegram not configured")?;
            let ch = TelegramChannel::new(tg.bot_token, tg.chat_id);
            ch.test_connection()
        }
        "discord" => {
            let dc = config
                .discord
                .ok_or("Discord not configured")?;
            let ch = DiscordChannel::new(dc.bot_token, dc.channel_id);
            ch.test_connection()
        }
        "feishu" => {
            let fs = config
                .feishu
                .ok_or("Feishu not configured")?;
            let ch = FeishuChannel::new(
                bot_id,
                fs.app_id,
                fs.app_secret,
                fs.chat_id,
                fs.reply_all_group_messages,
            );
            ch.test_connection()
        }
        _ => Err(format!("Unknown platform: {}", platform)),
    }
}

/// Start all enabled channel listeners for a bot.
#[tauri::command]
pub fn channel_start(
    app_handle: AppHandle,
    bot_id: String,
    state: State<'_, ChannelManagerState>,
) -> Result<(), String> {
    let mut active = state.active_channels.lock().unwrap();

    // Already running? Stop first.
    if let Some(handle) = active.remove(&bot_id) {
        handle.shutdown_flag.store(true, Ordering::Relaxed);
    }

    let config = read_channels_config(&bot_id)?;
    let channels = build_channels(&bot_id, &config);

    if channels.is_empty() {
        return Ok(());
    }

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let mut platforms = Vec::new();
    let mut conn_states: HashMap<String, Arc<AtomicU8>> = HashMap::new();

    // Start listener threads
    let (tx, rx) = mpsc::channel::<ChannelMessage>();

    for ch in &channels {
        let name = ch.name().to_string();
        if let Err(e) = ch.validate_config() {
            log::warn!("Channel {} validation failed: {}", name, e);
            continue;
        }
        let conn_state = Arc::new(AtomicU8::new(CONN_STATE_CONNECTING));
        if let Err(e) = ch.start_listener(tx.clone(), shutdown_flag.clone(), conn_state.clone()) {
            log::warn!("Channel {} listener start failed: {}", name, e);
            conn_state.store(CONN_STATE_DISCONNECTED, Ordering::Relaxed);
            continue;
        }
        conn_states.insert(name.clone(), conn_state);
        platforms.push(name);
    }

    // Drop the extra sender so rx will close when all listener senders are dropped
    drop(tx);

    // Spawn a forwarder thread that relays inbound messages to the frontend
    let bot_id_clone = bot_id.clone();
    let app = app_handle.clone();
    std::thread::spawn(move || {
        let event_name = format!("channel-incoming-{}", bot_id_clone);
        for msg in rx {
            log::info!("Channel forwarder: emitting {} for bot {}", event_name, bot_id_clone);
            if let Err(e) = app.emit(&event_name, &msg) {
                log::warn!("Failed to emit channel message: {}", e);
            }
        }
        log::info!("Channel forwarder for bot {} exited", bot_id_clone);
    });

    active.insert(
        bot_id.clone(),
        BotChannelHandle {
            shutdown_flag,
            platforms,
            conn_states,
        },
    );

    log::info!("Channels started for bot {}", bot_id);
    Ok(())
}

/// Stop all channel listeners for a bot.
#[tauri::command]
pub fn channel_stop(
    bot_id: String,
    state: State<'_, ChannelManagerState>,
) -> Result<(), String> {
    let mut active = state.active_channels.lock().unwrap();
    if let Some(handle) = active.remove(&bot_id) {
        handle.shutdown_flag.store(true, Ordering::Relaxed);
        log::info!("Channels stopped for bot {}", bot_id);
    }
    Ok(())
}

/// Get the status of channel listeners for a bot.
#[tauri::command]
pub fn channel_get_status(
    bot_id: String,
    state: State<'_, ChannelManagerState>,
) -> Result<Vec<ChannelStatus>, String> {
    let active = state.active_channels.lock().unwrap();
    let config = read_channels_config(&bot_id)?;

    let mut statuses = Vec::new();
    let handle = active.get(&bot_id);

    let all_platforms = ["telegram", "discord", "feishu"];
    let enabled = [
        config.telegram.as_ref().is_some_and(|t| t.enabled),
        config.discord.as_ref().is_some_and(|d| d.enabled),
        config.feishu.as_ref().is_some_and(|f| f.enabled),
    ];

    for (platform, is_enabled) in all_platforms.iter().zip(enabled.iter()) {
        if !is_enabled {
            continue;
        }

        let (connected, state_str) = match handle {
            Some(h) if h.platforms.contains(&platform.to_string()) => {
                let raw = h
                    .conn_states
                    .get(*platform)
                    .map(|s| s.load(Ordering::Relaxed))
                    .unwrap_or(CONN_STATE_DISCONNECTED);
                let state = match raw {
                    CONN_STATE_CONNECTING => "connecting",
                    CONN_STATE_CONNECTED => "connected",
                    _ => "disconnected",
                };
                (raw == CONN_STATE_CONNECTED, state)
            }
            _ => (false, "disconnected"),
        };

        statuses.push(ChannelStatus {
            platform: platform.to_string(),
            connected,
            state: state_str.to_string(),
            error: None,
        });
    }

    Ok(statuses)
}

/// Broadcast a message to all enabled channels for a bot.
/// `target_chat_id` is an optional dynamic target (e.g. Feishu source group chat_id).
#[tauri::command]
pub fn channel_send_message(
    bot_id: String,
    message: String,
    target_chat_id: Option<String>,
) -> Result<(), String> {
    let config = read_channels_config(&bot_id)?;
    let channels = build_channels(&bot_id, &config);

    if channels.is_empty() {
        return Ok(());
    }

    let msg = SendMessage {
        content: message,
        reply_to: None,
        target_chat_id,
    };

    let mut errors = Vec::new();
    for ch in &channels {
        if let Err(e) = ch.send(&msg) {
            log::warn!("Channel {} send failed: {}", ch.name(), e);
            errors.push(format!("{}: {}", ch.name(), e));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Some channels failed: {}", errors.join("; ")))
    }
}

/// Send a message to a single platform for a bot.
/// Used for platform-specific operational notices (e.g. Feishu queue acknowledgements).
#[tauri::command]
pub fn channel_send_platform_message(
    bot_id: String,
    platform: String,
    message: String,
    target_chat_id: Option<String>,
) -> Result<(), String> {
    let config = read_channels_config(&bot_id)?;
    let channels = build_channels(&bot_id, &config);

    if channels.is_empty() {
        return Ok(());
    }

    let msg = SendMessage {
        content: message,
        reply_to: None,
        target_chat_id,
    };

    let channel = channels
        .into_iter()
        .find(|ch| ch.name() == platform)
        .ok_or_else(|| format!("Platform not enabled for bot {}: {}", bot_id, platform))?;

    channel.send(&msg)
}
