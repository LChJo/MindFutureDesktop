//! Discord channel implementation.
//!
//! - Outbound: REST API `POST /channels/{id}/messages`
//! - Inbound: Gateway WebSocket (IDENTIFY → MESSAGE_CREATE events)

use super::{Channel, ChannelMessage, SendMessage, CONN_STATE_CONNECTED};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const API_BASE: &str = "https://discord.com/api/v10";
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

pub struct DiscordChannel {
    bot_token: String,
    channel_id: String,
    client: reqwest::blocking::Client,
}

impl DiscordChannel {
    pub fn new(bot_token: String, channel_id: String) -> Self {
        Self {
            bot_token,
            channel_id,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    fn validate_config(&self) -> Result<(), String> {
        if self.bot_token.is_empty() {
            return Err("Bot token is required".to_string());
        }
        if self.channel_id.is_empty() {
            return Err("Channel ID is required".to_string());
        }
        Ok(())
    }

    fn send(&self, msg: &SendMessage) -> Result<(), String> {
        let url = format!("{}/channels/{}/messages", API_BASE, self.channel_id);

        let body = serde_json::json!({
            "content": msg.content,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Discord send failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("Discord API error {}: {}", status, text));
        }

        Ok(())
    }

    fn start_listener(
        &self,
        tx: Sender<ChannelMessage>,
        shutdown: Arc<AtomicBool>,
        conn_state: Arc<AtomicU8>,
    ) -> Result<(), String> {
        conn_state.store(CONN_STATE_CONNECTED, Ordering::Relaxed);
        let token = self.bot_token.clone();
        let target_channel = self.channel_id.clone();

        std::thread::spawn(move || {
            log::info!("Discord listener started for channel {}", target_channel);

            // Reconnection loop
            while !shutdown.load(Ordering::Relaxed) {
                if let Err(e) =
                    discord_gateway_loop(&token, &target_channel, &tx, &shutdown)
                {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    log::warn!("Discord gateway error: {}, reconnecting in 5s...", e);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }

            log::info!("Discord listener stopped for channel {}", target_channel);
        });

        Ok(())
    }

    fn test_connection(&self) -> Result<(), String> {
        self.validate_config()?;

        let url = format!("{}/users/@me", API_BASE);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .map_err(|e| format!("Connection failed: {}", e))?;

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(format!("Invalid bot token: {}", text));
        }

        Ok(())
    }
}

/// Run a single Discord Gateway WebSocket session.
fn discord_gateway_loop(
    token: &str,
    target_channel: &str,
    tx: &Sender<ChannelMessage>,
    shutdown: &Arc<AtomicBool>,
) -> Result<(), String> {
    use tungstenite::connect;
    use tungstenite::Message;

    let (mut ws, _) =
        connect(GATEWAY_URL).map_err(|e| format!("WebSocket connect failed: {}", e))?;

    // Read HELLO to get heartbeat interval
    let hello = ws
        .read()
        .map_err(|e| format!("Failed to read HELLO: {}", e))?;
    let hello_data: serde_json::Value = match hello {
        Message::Text(ref t) => {
            serde_json::from_str(t).map_err(|e| format!("Failed to parse HELLO: {}", e))?
        }
        _ => return Err("Expected text message for HELLO".to_string()),
    };

    let heartbeat_interval = hello_data
        .get("d")
        .and_then(|d| d.get("heartbeat_interval"))
        .and_then(|h| h.as_u64())
        .unwrap_or(41250);

    // Send IDENTIFY
    let identify = serde_json::json!({
        "op": 2,
        "d": {
            "token": token,
            "intents": 512 | 32768, // GUILD_MESSAGES | MESSAGE_CONTENT
            "properties": {
                "os": "windows",
                "browser": "moonton-claude",
                "device": "moonton-claude"
            }
        }
    });
    ws.send(Message::Text(identify.to_string()))
        .map_err(|e| format!("Failed to send IDENTIFY: {}", e))?;

    // Set non-blocking for the WebSocket so we can check shutdown
    let stream = ws.get_ref();
    match stream {
        tungstenite::stream::MaybeTlsStream::NativeTls(ref s) => {
            let _ = s.get_ref().set_nonblocking(true);
        }
        tungstenite::stream::MaybeTlsStream::Plain(ref s) => {
            let _ = s.set_nonblocking(true);
        }
        _ => {}
    }

    let mut last_sequence: Option<u64> = None;
    let mut last_heartbeat = std::time::Instant::now();
    let heartbeat_dur = std::time::Duration::from_millis(heartbeat_interval);

    while !shutdown.load(Ordering::Relaxed) {
        // Send heartbeat if needed
        if last_heartbeat.elapsed() >= heartbeat_dur {
            let hb = serde_json::json!({
                "op": 1,
                "d": last_sequence,
            });
            if ws.send(Message::Text(hb.to_string())).is_err() {
                break;
            }
            last_heartbeat = std::time::Instant::now();
        }

        // Try to read a message (non-blocking)
        match ws.read() {
            Ok(Message::Text(text)) => {
                let data: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Track sequence number
                if let Some(s) = data.get("s").and_then(|s| s.as_u64()) {
                    last_sequence = Some(s);
                }

                let op = data.get("op").and_then(|o| o.as_u64()).unwrap_or(0);

                match op {
                    // Heartbeat ACK
                    11 => {}
                    // Heartbeat request
                    1 => {
                        let hb = serde_json::json!({ "op": 1, "d": last_sequence });
                        let _ = ws.send(Message::Text(hb.to_string()));
                        last_heartbeat = std::time::Instant::now();
                    }
                    // Dispatch
                    0 => {
                        let event_name = data
                            .get("t")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");

                        if event_name == "MESSAGE_CREATE" {
                            if let Some(d) = data.get("d") {
                                // Check channel
                                let msg_channel = d
                                    .get("channel_id")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("");

                                if msg_channel != target_channel {
                                    continue;
                                }

                                // Skip bot messages
                                let is_bot = d
                                    .get("author")
                                    .and_then(|a| a.get("bot"))
                                    .and_then(|b| b.as_bool())
                                    .unwrap_or(false);
                                if is_bot {
                                    continue;
                                }

                                let content = d
                                    .get("content")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                if content.is_empty() {
                                    continue;
                                }

                                let sender = d
                                    .get("author")
                                    .and_then(|a| a.get("username"))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string();

                                let timestamp = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                                    as u64;

                                let channel_msg = ChannelMessage {
                                    platform: "discord".to_string(),
                                    channel_id: target_channel.to_string(),
                                    sender,
                                    content,
                                    timestamp,
                                    attachments: Vec::new(),
                                    message_id: String::new(),
                                };

                                if tx.send(channel_msg).is_err() {
                                    log::info!(
                                        "Discord listener: receiver dropped, exiting"
                                    );
                                    return Ok(());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Message::Close(_)) => {
                return Err("WebSocket closed by server".to_string());
            }
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                // No data available, sleep briefly
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("WebSocket error: {}", e));
            }
            _ => {}
        }
    }

    let _ = ws.close(None);
    Ok(())
}
