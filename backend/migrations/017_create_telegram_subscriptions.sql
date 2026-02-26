-- Telegram bot subscription tracking
CREATE TABLE IF NOT EXISTS telegram_subscriptions (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id INTEGER NOT NULL UNIQUE,
    chat_type TEXT NOT NULL DEFAULT 'private',
    chat_title TEXT,
    username TEXT,
    subscribed_at TEXT NOT NULL DEFAULT (datetime('now')),
    is_active INTEGER NOT NULL DEFAULT 1,
    alert_types TEXT NOT NULL DEFAULT 'all',
    last_alert_sent_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_telegram_subscriptions_chat_id ON telegram_subscriptions(chat_id);
CREATE INDEX IF NOT EXISTS idx_telegram_subscriptions_active ON telegram_subscriptions(is_active);
