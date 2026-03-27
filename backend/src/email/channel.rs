use crate::broadcast::{Message, NotificationChannel};
use crate::email::service::EmailService;
use async_trait::async_trait;
use std::sync::Arc;

pub struct EmailChannel {
    service: Arc<EmailService>,
    recipients: Vec<String>,
}

impl EmailChannel {
    #[must_use]
    pub const fn new(service: Arc<EmailService>, recipients: Vec<String>) -> Self {
        Self {
            service,
            recipients,
        }
    }
}

#[async_trait]
impl NotificationChannel for EmailChannel {
    fn name(&self) -> &'static str {
        "email"
    }

    async fn send(&self, message: Message) -> anyhow::Result<()> {
        let html = if let Some(metadata) = &message.metadata {
            format!(
                "<h2>{}</h2><p>{}</p><pre>{}</pre>",
                message.subject, message.body, metadata
            )
        } else {
            format!("<h2>{}</h2><p>{}</p>", message.subject, message.body)
        };

        for recipient in &self.recipients {
            self.service.send_html(recipient, &message.subject, &html)?;
        }
        Ok(())
    }
}
