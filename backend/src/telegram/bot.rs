use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::alerts::{Alert, AlertManager};
use crate::cache::CacheManager;
use crate::database::Database;
use crate::rpc::StellarRpcClient;
use crate::telegram::client::{BotCommand, TelegramClient};
use crate::telegram::commands::CommandHandler;
use crate::telegram::formatter;
use crate::telegram::subscription::SubscriptionService;

pub struct TelegramBot {
    client: Arc<TelegramClient>,
    command_handler: Arc<CommandHandler>,
    subscriptions: Arc<SubscriptionService>,
    alert_rx: broadcast::Receiver<Alert>,
}

impl TelegramBot {
    pub fn new(
        token: &str,
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        rpc_client: Arc<StellarRpcClient>,
        subscriptions: Arc<SubscriptionService>,
        alert_manager: &AlertManager,
    ) -> Self {
        let client = Arc::new(TelegramClient::new(token));
        let command_handler = Arc::new(CommandHandler::new(
            db,
            cache,
            rpc_client,
            Arc::clone(&subscriptions),
        ));
        let alert_rx = alert_manager.subscribe();

        Self {
            client,
            command_handler,
            subscriptions,
            alert_rx,
        }
    }

    pub async fn run(self, mut shutdown_rx: broadcast::Receiver<()>) {
        let client = self.client;
        let command_handler = self.command_handler;
        let subscriptions = self.subscriptions;
        let alert_rx = self.alert_rx;

        // Register bot commands on startup
        if let Err(e) = register_commands(&client).await {
            tracing::warn!("Failed to register Telegram bot commands: {}", e);
        } else {
            tracing::info!("Telegram bot commands registered");
        }

        // Spawn polling task
        let poll_client = Arc::clone(&client);
        let poll_handler = Arc::clone(&command_handler);
        let poll_shutdown = shutdown_rx.resubscribe();
        let poll_task = tokio::spawn(async move {
            polling_loop(poll_client, poll_handler, poll_shutdown).await;
        });

        // Spawn alert forwarding task
        let alert_client = Arc::clone(&client);
        let alert_subs = Arc::clone(&subscriptions);
        let alert_shutdown = shutdown_rx.resubscribe();
        let alert_task = tokio::spawn(async move {
            alert_loop(alert_client, alert_subs, alert_rx, alert_shutdown).await;
        });

        // Wait for shutdown signal
        let _ = shutdown_rx.recv().await;
        tracing::info!("Telegram bot received shutdown signal");

        // Wait briefly for tasks to finish
        let _ = tokio::time::timeout(Duration::from_secs(5), async {
            let _ = poll_task.await;
            let _ = alert_task.await;
        })
        .await;

        tracing::info!("Telegram bot shut down");
    }
}

/// Parse a Telegram command, stripping the optional @bot_name suffix.
fn parse_command(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    if !text.starts_with('/') {
        return None;
    }

    let (cmd_part, args) = match text.find(' ') {
        Some(pos) => (&text[1..pos], text[pos + 1..].trim()),
        None => (&text[1..], ""),
    };

    // Strip @bot_name suffix
    let command = match cmd_part.find('@') {
        Some(pos) => &cmd_part[..pos],
        None => cmd_part,
    };

    Some((command, args))
}

async fn polling_loop(
    client: Arc<TelegramClient>,
    handler: Arc<CommandHandler>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut offset: Option<i64> = None;

    tracing::info!("Telegram bot polling started");

    loop {
        tokio::select! {
            result = client.get_updates(offset) => {
                match result {
                    Ok(updates) => {
                        for update in updates {
                            // Always advance offset
                            offset = Some(update.update_id + 1);

                            if let Some(message) = &update.message {
                                if let Some(text) = &message.text {
                                    if let Some((command, args)) = parse_command(text) {
                                        let chat_id = message.chat.id;
                                        let chat_type = &message.chat.chat_type;
                                        let chat_title = message.chat.title.as_deref();
                                        let username = message
                                            .from
                                            .as_ref()
                                            .and_then(|u| u.username.as_deref());

                                        let response = handler
                                            .handle_command(
                                                command,
                                                args,
                                                chat_id,
                                                chat_type,
                                                chat_title,
                                                username,
                                            )
                                            .await;

                                        if let Err(e) = client.send_message(chat_id, &response).await {
                                            tracing::error!(
                                                "Failed to send Telegram message to {}: {}",
                                                chat_id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Telegram getUpdates error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Telegram polling loop shutting down");
                break;
            }
        }
    }
}

async fn alert_loop(
    client: Arc<TelegramClient>,
    subscriptions: Arc<SubscriptionService>,
    mut alert_rx: broadcast::Receiver<Alert>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    tracing::info!("Telegram alert forwarding started");

    loop {
        tokio::select! {
            result = alert_rx.recv() => {
                match result {
                    Ok(alert) => {
                        let message = formatter::format_alert(&alert);

                        match subscriptions.get_active_chat_ids().await {
                            Ok(chat_ids) => {
                                for chat_id in chat_ids {
                                    if let Err(e) = client.send_message(chat_id, &message).await {
                                        tracing::error!(
                                            "Failed to send alert to Telegram chat {}: {}",
                                            chat_id,
                                            e
                                        );
                                    } else {
                                        let _ = subscriptions.update_last_alert_sent(chat_id).await;
                                    }
                                    // Rate limit: 50ms between sends
                                    tokio::time::sleep(Duration::from_millis(50)).await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to get active Telegram subscribers: {}", e);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Telegram alert receiver lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Alert channel closed, stopping alert loop");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Telegram alert loop shutting down");
                break;
            }
        }
    }
}

async fn register_commands(client: &TelegramClient) -> anyhow::Result<()> {
    let commands = vec![
        BotCommand {
            command: "start".to_string(),
            description: "Get started with the bot".to_string(),
        },
        BotCommand {
            command: "help".to_string(),
            description: "Show available commands".to_string(),
        },
        BotCommand {
            command: "status".to_string(),
            description: "System health summary".to_string(),
        },
        BotCommand {
            command: "corridors".to_string(),
            description: "Top corridors with metrics".to_string(),
        },
        BotCommand {
            command: "corridor".to_string(),
            description: "Detailed corridor info".to_string(),
        },
        BotCommand {
            command: "anchors".to_string(),
            description: "List anchors with reliability".to_string(),
        },
        BotCommand {
            command: "anchor".to_string(),
            description: "Detailed anchor info".to_string(),
        },
        BotCommand {
            command: "subscribe".to_string(),
            description: "Subscribe to alerts".to_string(),
        },
        BotCommand {
            command: "unsubscribe".to_string(),
            description: "Unsubscribe from alerts".to_string(),
        },
    ];

    client.set_my_commands(&commands).await
}
