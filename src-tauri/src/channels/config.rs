//! Channel configuration persistence.
//!
//! Each bot stores its channel configuration in
//! `~/.MoontonClaudeDesktop/bots/{botId}/channels.json`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Top-level channel configuration for a bot.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BotChannelsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<TelegramConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord: Option<DiscordConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feishu: Option<FeishuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    #[serde(rename = "botToken")]
    pub bot_token: String,
    #[serde(rename = "chatId")]
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    #[serde(rename = "botToken")]
    pub bot_token: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub enabled: bool,
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "appSecret")]
    pub app_secret: String,
    #[serde(rename = "chatId", default)]
    pub chat_id: String,
    /// When true, respond to ALL group messages; when false (default),
    /// only respond to @bot mentions in groups. P2P always processed.
    #[serde(rename = "replyAllGroupMessages", default)]
    pub reply_all_group_messages: bool,
}

// ---------------------------------------------------------------------------
// Path helpers (reuses the same data directory as claude_bot.rs)
// ---------------------------------------------------------------------------

fn get_bot_dir(bot_id: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    let bot_dir = home
        .join(".MoontonClaudeDesktop")
        .join("bots")
        .join(bot_id);
    if !bot_dir.exists() {
        fs::create_dir_all(&bot_dir)
            .map_err(|e| format!("Failed to create bot directory: {}", e))?;
    }
    Ok(bot_dir)
}

fn channels_config_path(bot_id: &str) -> Result<PathBuf, String> {
    Ok(get_bot_dir(bot_id)?.join("channels.json"))
}

// ---------------------------------------------------------------------------
// Read / Write
// ---------------------------------------------------------------------------

pub fn read_channels_config(bot_id: &str) -> Result<BotChannelsConfig, String> {
    let path = channels_config_path(bot_id)?;
    if !path.exists() {
        return Ok(BotChannelsConfig::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read channels config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse channels config: {}", e))
}

pub fn write_channels_config(bot_id: &str, config: &BotChannelsConfig) -> Result<(), String> {
    let path = channels_config_path(bot_id)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize channels config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write channels config: {}", e))
}
