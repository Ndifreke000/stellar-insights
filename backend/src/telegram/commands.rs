use std::sync::Arc;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::rpc::StellarRpcClient;
use crate::telegram::formatter;
use crate::telegram::subscription::SubscriptionService;

pub struct CommandHandler {
    db: Arc<Database>,
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
    subscriptions: Arc<SubscriptionService>,
}

impl CommandHandler {
    pub fn new(
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        rpc_client: Arc<StellarRpcClient>,
        subscriptions: Arc<SubscriptionService>,
    ) -> Self {
        Self {
            db,
            cache,
            rpc_client,
            subscriptions,
        }
    }

    pub async fn handle_command(
        &self,
        command: &str,
        args: &str,
        chat_id: i64,
        chat_type: &str,
        chat_title: Option<&str>,
        username: Option<&str>,
    ) -> String {
        match command {
            "start" | "help" => formatter::format_help(),
            "status" => self.handle_status().await,
            "corridors" => self.handle_corridors().await,
            "corridor" => self.handle_corridor_detail(args).await,
            "anchors" => self.handle_anchors().await,
            "anchor" => self.handle_anchor_detail(args).await,
            "subscribe" => {
                self.handle_subscribe(chat_id, chat_type, chat_title, username)
                    .await
            }
            "unsubscribe" => self.handle_unsubscribe(chat_id).await,
            _ => formatter::escape_markdown("Unknown command. Use /help for available commands."),
        }
    }

    async fn handle_status(&self) -> String {
        let anchors = match self.db.list_anchors(1000, 0).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to fetch anchors for status: {}", e);
                vec![]
            }
        };
        let anchor_count = anchors.len();

        let corridor_count = match self.rpc_client.fetch_payments(200, None).await {
            Ok(payments) => {
                let mut corridors = std::collections::HashSet::new();
                for p in &payments {
                    let key = format!("{}->XLM", p.asset_code.as_deref().unwrap_or("XLM"));
                    corridors.insert(key);
                }
                corridors.len()
            }
            Err(_) => 0,
        };

        formatter::format_status(corridor_count, anchor_count, 0)
    }

    async fn handle_corridors(&self) -> String {
        let payments = match self.rpc_client.fetch_payments(200, None).await {
            Ok(p) => p,
            Err(e) => {
                return formatter::escape_markdown(&format!(
                    "Failed to fetch corridor data: {}",
                    e
                ));
            }
        };

        let mut corridor_map: std::collections::HashMap<String, (i64, f64)> =
            std::collections::HashMap::new();

        for payment in &payments {
            let key = format!(
                "{}:{}->XLM:native",
                payment.get_asset_code().as_deref().unwrap_or("XLM"),
                payment.get_asset_issuer().as_deref().unwrap_or("native")
            );
            let amount: f64 = payment.get_amount().parse().unwrap_or(0.0);
            let entry = corridor_map.entry(key).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += amount;
        }

        let mut corridors: Vec<(String, f64, i64, f64)> = corridor_map
            .into_iter()
            .map(|(id, (count, _volume))| {
                let success_rate = 100.0;
                let health = 95.0;
                (id, success_rate, count, health)
            })
            .collect();

        corridors.sort_by(|a, b| b.2.cmp(&a.2));
        corridors.truncate(10);

        formatter::format_corridor_list(&corridors)
    }

    async fn handle_corridor_detail(&self, args: &str) -> String {
        let key = args.trim();
        if key.is_empty() {
            return formatter::escape_markdown(
                "Usage: /corridor <corridor_key>\nExample: /corridor USDC:GA5Z->XLM:native",
            );
        }

        let payments = match self.rpc_client.fetch_payments(200, None).await {
            Ok(p) => p,
            Err(e) => {
                return formatter::escape_markdown(&format!(
                    "Failed to fetch corridor data: {}",
                    e
                ));
            }
        };

        let mut count: i64 = 0;
        let mut volume: f64 = 0.0;

        for payment in &payments {
            let corridor_key = format!(
                "{}:{}->XLM:native",
                payment.get_asset_code().as_deref().unwrap_or("XLM"),
                payment.get_asset_issuer().as_deref().unwrap_or("native")
            );
            if corridor_key == key {
                count += 1;
                volume += payment.get_amount().parse::<f64>().unwrap_or(0.0);
            }
        }

        if count == 0 {
            return formatter::escape_markdown(&format!("Corridor '{}' not found.", key));
        }

        let parts: Vec<&str> = key.split("->").collect();
        let (src, dst) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (key, "unknown")
        };

        formatter::format_corridor_detail(key, src, dst, 100.0, count, 400.0, volume, 95.0)
    }

    async fn handle_anchors(&self) -> String {
        let anchors = match self.db.list_anchors(50, 0).await {
            Ok(a) => a,
            Err(e) => {
                return formatter::escape_markdown(&format!("Failed to fetch anchors: {}", e));
            }
        };

        let anchor_data: Vec<(String, String, f64, String)> = anchors
            .into_iter()
            .map(|a| (a.id, a.name, a.reliability_score, a.status))
            .collect();

        formatter::format_anchor_list(&anchor_data)
    }

    async fn handle_anchor_detail(&self, args: &str) -> String {
        let id = args.trim();
        if id.is_empty() {
            return formatter::escape_markdown(
                "Usage: /anchor <anchor_id>\nExample: /anchor 550e8400-e29b-41d4-a716-446655440000",
            );
        }

        let anchor_id = match uuid::Uuid::parse_str(id) {
            Ok(u) => u,
            Err(_) => {
                return formatter::escape_markdown("Invalid anchor ID format. Must be a UUID.");
            }
        };

        match self.db.get_anchor_by_id(anchor_id).await {
            Ok(Some(anchor)) => formatter::format_anchor_detail(
                &anchor.name,
                &anchor.stellar_account,
                anchor.reliability_score,
                anchor.total_transactions,
                anchor.successful_transactions,
                anchor.failed_transactions,
                &anchor.status,
            ),
            Ok(None) => formatter::escape_markdown(&format!("Anchor '{}' not found.", id)),
            Err(e) => formatter::escape_markdown(&format!("Failed to fetch anchor: {}", e)),
        }
    }

    async fn handle_subscribe(
        &self,
        chat_id: i64,
        chat_type: &str,
        chat_title: Option<&str>,
        username: Option<&str>,
    ) -> String {
        match self
            .subscriptions
            .subscribe(chat_id, chat_type, chat_title, username)
            .await
        {
            Ok(true) => formatter::escape_markdown(
                "Subscribed to alerts! You will receive notifications when corridor health changes.",
            ),
            Ok(false) => {
                formatter::escape_markdown("You are already subscribed to alerts.")
            }
            Err(e) => formatter::escape_markdown(&format!("Failed to subscribe: {}", e)),
        }
    }

    async fn handle_unsubscribe(&self, chat_id: i64) -> String {
        match self.subscriptions.unsubscribe(chat_id).await {
            Ok(true) => formatter::escape_markdown(
                "Unsubscribed from alerts. You will no longer receive notifications.",
            ),
            Ok(false) => formatter::escape_markdown("You are not currently subscribed to alerts."),
            Err(e) => formatter::escape_markdown(&format!("Failed to unsubscribe: {}", e)),
        }
    }
}
