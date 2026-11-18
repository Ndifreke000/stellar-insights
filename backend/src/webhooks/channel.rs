use crate::broadcast::{Message, NotificationChannel};
use crate::rpc::circuit_breaker::{integration_circuit_breaker, SharedCircuitBreaker};
use crate::webhooks::WebhookSignature;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WebhookEndpoint {
    pub url: String,
    pub secret: Option<String>,
    pub headers: HashMap<String, String>,
}

impl WebhookEndpoint {
    #[must_use]
    pub fn new(url: String, secret: Option<String>) -> Self {
        Self {
            url,
            secret,
            headers: HashMap::new(),
        }
    }
}

pub struct WebhookChannel {
    client: Client,
    endpoints: Vec<WebhookEndpoint>,
    breaker: SharedCircuitBreaker,
}

impl WebhookChannel {
    #[must_use]
    pub fn new(endpoints: Vec<WebhookEndpoint>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            endpoints,
            breaker: integration_circuit_breaker("webhook_channel"),
        }
    }
}

#[async_trait]
impl NotificationChannel for WebhookChannel {
    fn name(&self) -> &'static str {
        "webhook"
    }

    async fn send(&self, message: Message) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "subject": message.subject,
            "body": message.body,
            "metadata": message.metadata,
            "timestamp": chrono::Utc::now().timestamp(),
        });
        let payload_str = serde_json::to_string(&payload)?;

        for endpoint in &self.endpoints {
            let mut request = self
                .client
                .post(&endpoint.url)
                .header("Content-Type", "application/json");

            if let Some(secret) = &endpoint.secret {
                let signature = WebhookSignature::sign(&payload_str, secret);
                request = request.header("X-Signature", signature);
            }

            for (name, value) in &endpoint.headers {
                request = request.header(name, value);
            }

            let url = endpoint.url.clone();
            let body = payload_str.clone();
            self.breaker
                .call(async move {
                    let response = request.body(body).send().await?;
                    if !response.status().is_success() {
                        anyhow::bail!("webhook endpoint {} returned {}", url, response.status());
                    }
                    Ok(())
                })
                .await
                .map_err(crate::rpc::CallError::into_anyhow)?;
        }
        Ok(())
    }
}
