//! Telegram channel implementation using the Bot API (pure HTTP).

use super::{Channel, ChannelMessage, SendMessage, CONN_STATE_CONNECTED};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const API_BASE: &str = "https://api.telegram.org/bot";

pub struct TelegramChannel {
    bot_token: String,
    chat_id: String,
    client: reqwest::blocking::Client,
}

impl TelegramChannel {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}{}/{}", API_BASE, self.bot_token, method)
    }
}

impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    fn validate_config(&self) -> Result<(), String> {
        if self.bot_token.is_empty() {
            return Err("Bot token is required".to_string());
        }
        if self.chat_id.is_empty() {
            return Err("Chat ID is required".to_string());
        }
        Ok(())
    }

    fn send(&self, msg: &SendMessage) -> Result<(), String> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": msg.content,
            "parse_mode": "Markdown",
        });

        let resp = self
            .client
            .post(&self.api_url("sendMessage"))
            .json(&body)
            .send()
            .map_err(|e| format!("Telegram send failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("Telegram API error {}: {}", status, text));
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
        let chat_id = self.chat_id.clone();

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        std::thread::spawn(move || {
            // Flush pending updates: fetch latest update_id and skip everything before it
            let mut offset: i64 = {
                let flush_url = format!(
                    "{}{}/getUpdates?offset=-1&limit=1",
                    API_BASE, token
                );
                match client.get(&flush_url).send() {
                    Ok(resp) => {
                        if let Ok(body) = resp.json::<serde_json::Value>() {
                            body.get("result")
                                .and_then(|r| r.as_array())
                                .and_then(|arr| arr.last())
                                .and_then(|u| u.get("update_id"))
                                .and_then(|id| id.as_i64())
                                .map(|id| id + 1)
                                .unwrap_or(0)
                        } else {
                            0
                        }
                    }
                    Err(_) => 0,
                }
            };

            log::info!("Telegram listener started for chat {} (offset={})", chat_id, offset);

            while !shutdown.load(Ordering::Relaxed) {
                let url = format!(
                    "{}{}/getUpdates?offset={}&timeout=30",
                    API_BASE, token, offset
                );

                let resp = match client.get(&url).send() {
                    Ok(r) => r,
                    Err(e) => {
                        if shutdown.load(Ordering::Relaxed) {
                            break;
                        }
                        log::warn!("Telegram poll error: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }
                };

                let body: serde_json::Value = match resp.json() {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Telegram parse error: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }
                };

                if let Some(results) = body.get("result").and_then(|r| r.as_array()) {
                    for update in results {
                        // Update offset
                        if let Some(uid) = update.get("update_id").and_then(|u| u.as_i64()) {
                            offset = uid + 1;
                        }

                        // Extract message
                        let message = match update.get("message") {
                            Some(m) => m,
                            None => continue,
                        };

                        // Only process messages from the target chat
                        let msg_chat_id = message
                            .get("chat")
                            .and_then(|c| c.get("id"))
                            .and_then(|id| id.as_i64())
                            .map(|id| id.to_string())
                            .unwrap_or_default();

                        if msg_chat_id != chat_id {
                            continue;
                        }

                        // Skip bot messages
                        let is_bot = message
                            .get("from")
                            .and_then(|f| f.get("is_bot"))
                            .and_then(|b| b.as_bool())
                            .unwrap_or(false);
                        if is_bot {
                            continue;
                        }

                        let text = message
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        if text.is_empty() {
                            continue;
                        }

                        let sender = message
                            .get("from")
                            .and_then(|f| f.get("first_name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown")
                            .to_string();

                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        let channel_msg = ChannelMessage {
                            platform: "telegram".to_string(),
                            channel_id: chat_id.clone(),
                            sender,
                            content: text,
                            timestamp,
                            attachments: Vec::new(),
                            message_id: String::new(),
                        };

                        if tx.send(channel_msg).is_err() {
                            log::info!("Telegram listener: receiver dropped, exiting");
                            return;
                        }
                    }
                }
            }

            log::info!("Telegram listener stopped for chat {}", chat_id);
        });

        Ok(())
    }

    fn test_connection(&self) -> Result<(), String> {
        self.validate_config()?;

        let resp = self
            .client
            .get(&self.api_url("getMe"))
            .send()
            .map_err(|e| format!("Connection failed: {}", e))?;

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(format!("Invalid bot token: {}", text));
        }

        Ok(())
    }
}
