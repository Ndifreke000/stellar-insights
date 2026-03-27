use crate::broadcast::{Message, NotificationChannel};
use crate::telegram::client::TelegramClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TelegramChannel {
    client: Arc<TelegramClient>,
    chat_ids: Vec<i64>,
}

impl TelegramChannel {
    #[must_use]
    pub const fn new(client: Arc<TelegramClient>, chat_ids: Vec<i64>) -> Self {
        Self { client, chat_ids }
    }
}

#[async_trait]
impl NotificationChannel for TelegramChannel {
    fn name(&self) -> &'static str {
        "telegram"
    }

    async fn send(&self, message: Message) -> anyhow::Result<()> {
        let text = format!("*{}*\n\n{}", message.subject, message.body);
        for chat_id in &self.chat_ids {
            self.client.send_message(*chat_id, &text).await?;
        }
        Ok(())
    }
}
