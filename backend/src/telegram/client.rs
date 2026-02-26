use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Telegram Bot API client using reqwest directly.
pub struct TelegramClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub from: Option<User>,
    pub text: Option<String>,
    pub date: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub title: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BotCommand {
    pub command: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    chat_id: i64,
    text: &'a str,
    parse_mode: &'a str,
}

#[derive(Debug, Serialize)]
struct GetUpdatesRequest {
    offset: Option<i64>,
    timeout: i64,
    allowed_updates: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SetMyCommandsRequest<'a> {
    commands: &'a [BotCommand],
}

impl TelegramClient {
    pub fn new(token: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(35))
            .build()
            .expect("Failed to build Telegram HTTP client");

        Self {
            client,
            base_url: format!("https://api.telegram.org/bot{}", token),
        }
    }

    pub async fn get_updates(&self, offset: Option<i64>) -> anyhow::Result<Vec<Update>> {
        let body = GetUpdatesRequest {
            offset,
            timeout: 30,
            allowed_updates: vec!["message".to_string()],
        };

        let resp: TelegramResponse<Vec<Update>> = self
            .client
            .post(format!("{}/getUpdates", self.base_url))
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            anyhow::bail!(
                "Telegram getUpdates failed: {}",
                resp.description.unwrap_or_default()
            );
        }

        Ok(resp.result.unwrap_or_default())
    }

    pub async fn send_message(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let body = SendMessageRequest {
            chat_id,
            text,
            parse_mode: "MarkdownV2",
        };

        let resp: TelegramResponse<serde_json::Value> = self
            .client
            .post(format!("{}/sendMessage", self.base_url))
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            // Retry without parse mode if MarkdownV2 fails
            let fallback = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            });
            let resp2: TelegramResponse<serde_json::Value> = self
                .client
                .post(format!("{}/sendMessage", self.base_url))
                .json(&fallback)
                .send()
                .await?
                .json()
                .await?;

            if !resp2.ok {
                anyhow::bail!(
                    "Telegram sendMessage failed: {}",
                    resp2.description.unwrap_or_default()
                );
            }
        }

        Ok(())
    }

    pub async fn set_my_commands(&self, commands: &[BotCommand]) -> anyhow::Result<()> {
        let body = SetMyCommandsRequest { commands };

        let resp: TelegramResponse<bool> = self
            .client
            .post(format!("{}/setMyCommands", self.base_url))
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            anyhow::bail!(
                "Telegram setMyCommands failed: {}",
                resp.description.unwrap_or_default()
            );
        }

        Ok(())
    }
}
