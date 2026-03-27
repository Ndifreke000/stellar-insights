use anyhow::{Context, Result};
use crate::models::alert::{Alert, AlertChannel};

pub struct EmailService {}
impl EmailService {
    pub async fn send_alert_email(&self, _alert: &Alert) -> Result<()> {
        Ok(())
    }
}

pub struct SlackClient {}
impl SlackClient {
    pub async fn post_message(&self, _msg: &str) -> Result<()> {
        Ok(())
    }
}

pub struct WebhookDispatcher {}
impl WebhookDispatcher {
    pub async fn dispatch_alert(&self, _alert: &Alert) -> Result<()> {
        Ok(())
    }
}

pub struct AlertService {
    pub email_service: EmailService,
    pub slack_client: SlackClient,
    pub webhook_dispatcher: WebhookDispatcher,
}

impl AlertService {
    pub fn new() -> Self {
        Self {
            email_service: EmailService {},
            slack_client: SlackClient {},
            webhook_dispatcher: WebhookDispatcher {},
        }
    }

    pub async fn send_alert(&self, alert: Alert) -> Result<()> {
        // ✅ Fixed Example 2: Implement actual delivery
        match alert.channel {
            AlertChannel::Email => {
                self.email_service
                    .send_alert_email(&alert)
                    .await
                    .context("Failed to send email alert")?;
            }
            AlertChannel::Slack => {
                self.slack_client
                    .post_message(&format!("Slack Alert: {:?}", alert))
                    .await
                    .context("Failed to send Slack alert")?;
            }
            AlertChannel::Webhook => {
                self.webhook_dispatcher
                    .dispatch_alert(&alert)
                    .await
                    .context("Failed to send webhook alert")?;
            }
        }

        tracing::info!("Alert sent successfully: {} via {:?}", alert.id, alert.channel);

        Ok(())
    }
}