//! Channel system for forwarding bot conversations to social platforms.
//!
//! Supports bidirectional messaging with Telegram, Discord, and Feishu (Lark).

pub mod config;
pub mod discord;
pub mod feishu;
pub mod manager;
pub mod telegram;

pub use feishu::{feishu_add_reaction, feishu_clear_app_reactions, feishu_remove_reaction};
pub use manager::{
    channel_get_config, channel_get_status, channel_save_config, channel_send_message,
    channel_send_platform_message, channel_start, channel_stop, channel_test_connection,
    ChannelManagerState,
};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Connection state constants for channels with async connection (e.g. WebSocket).
pub const CONN_STATE_DISCONNECTED: u8 = 0;
pub const CONN_STATE_CONNECTING: u8 = 1;
pub const CONN_STATE_CONNECTED: u8 = 2;

/// Attachment (downloaded file) from a social platform message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAttachment {
    pub path: String,
    pub name: String,
}

/// Inbound message received from a social platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub platform: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<ChannelAttachment>,
    /// Platform-specific message ID (e.g. Feishu message_id) for reaction management.
    #[serde(rename = "messageId", default, skip_serializing_if = "String::is_empty")]
    pub message_id: String,
}

/// Outbound message to send to social platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessage {
    pub content: String,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// Dynamic target chat ID (used by Feishu to reply to the source group/DM).
    #[serde(rename = "targetChatId", skip_serializing_if = "Option::is_none")]
    pub target_chat_id: Option<String>,
}

/// Trait implemented by each platform channel (Telegram, Discord, Feishu).
pub trait Channel: Send + Sync {
    /// Platform name (e.g. "telegram", "discord", "feishu").
    fn name(&self) -> &str;

    /// Validate that the configuration is complete and well-formed.
    fn validate_config(&self) -> Result<(), String>;

    /// Send a message to the platform.
    fn send(&self, msg: &SendMessage) -> Result<(), String>;

    /// Start a background listener that pushes inbound messages to `tx`.
    /// The listener should check `shutdown` periodically and exit when it becomes `true`.
    /// `conn_state` tracks the real-time connection state (connecting/connected/disconnected).
    fn start_listener(
        &self,
        tx: Sender<ChannelMessage>,
        shutdown: Arc<AtomicBool>,
        conn_state: Arc<AtomicU8>,
    ) -> Result<(), String>;

    /// Test the connection / credentials. Returns Ok(()) on success.
    fn test_connection(&self) -> Result<(), String>;
}
