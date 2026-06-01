use crate::alerts::{Alert, AlertType};

/// Escape special characters for Telegram MarkdownV2.
pub fn escape_markdown(text: &str) -> String {
    let special = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut escaped = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if special.contains(&ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

pub fn format_alert(alert: &Alert) -> String {
    let emoji = match alert.alert_type {
        AlertType::SuccessRateDrop => "\u{1F534}",   // red circle
        AlertType::LatencyIncrease => "\u{1F7E1}",   // yellow circle
        AlertType::LiquidityDecrease => "\u{1F7E0}", // orange circle
    };

    let type_label = match alert.alert_type {
        AlertType::SuccessRateDrop => "Success Rate Drop",
        AlertType::LatencyIncrease => "Latency Increase",
        AlertType::LiquidityDecrease => "Liquidity Decrease",
    };

    let corridor = escape_markdown(&alert.corridor_id);
    let message = escape_markdown(&alert.message);
    let ts = escape_markdown(&alert.timestamp);

    format!(
        "{emoji} *{type_label}*\n\
         Corridor: `{corridor}`\n\
         {message}\n\
         Time: {ts}",
        emoji = emoji,
        type_label = escape_markdown(type_label),
        corridor = corridor,
        message = message,
        ts = ts,
    )
}

pub fn format_status(corridor_count: usize, anchor_count: usize, active_alerts: usize) -> String {
    let title = escape_markdown("System Status");
    format!(
        "*{title}*\n\n\
         Corridors: {corridors}\n\
         Anchors: {anchors}\n\
         Active Alerts: {alerts}",
        title = title,
        corridors = corridor_count,
        anchors = anchor_count,
        alerts = active_alerts,
    )
}

pub fn format_corridor_list(
    corridors: &[(String, f64, i64, f64)], // (id, success_rate, volume, health_score)
) -> String {
    if corridors.is_empty() {
        return escape_markdown("No corridors found.");
    }

    let title = escape_markdown("Top Corridors");
    let mut lines = vec![format!("*{title}*\n")];

    for (i, (id, success_rate, volume, health)) in corridors.iter().enumerate() {
        let health_emoji = if *health >= 90.0 {
            "\u{2705}" // green check
        } else if *health >= 70.0 {
            "\u{26A0}\u{FE0F}" // warning
        } else {
            "\u{274C}" // red X
        };

        lines.push(format!(
            "{health_emoji} `{id}`\n   Rate: {sr:.1}% \\| Vol: {vol} \\| Health: {h:.0}",
            health_emoji = health_emoji,
            id = escape_markdown(id),
            sr = success_rate,
            vol = volume,
            h = health,
        ));

        if i >= 9 {
            break;
        }
    }

    lines.join("\n")
}

pub fn format_corridor_detail(
    id: &str,
    source_asset: &str,
    dest_asset: &str,
    success_rate: f64,
    total_attempts: i64,
    avg_latency: f64,
    liquidity_usd: f64,
    health_score: f64,
) -> String {
    let title = escape_markdown(&format!("Corridor: {}", id));
    format!(
        "*{title}*\n\n\
         Source: `{src}`\n\
         Destination: `{dst}`\n\
         Success Rate: {sr:.1}%\n\
         Total Attempts: {ta}\n\
         Avg Latency: {lat:.0}ms\n\
         Liquidity: ${liq:.0}\n\
         Health Score: {hs:.1}",
        title = title,
        src = escape_markdown(source_asset),
        dst = escape_markdown(dest_asset),
        sr = success_rate,
        ta = total_attempts,
        lat = avg_latency,
        liq = liquidity_usd,
        hs = health_score,
    )
}

pub fn format_anchor_list(
    anchors: &[(String, String, f64, String)], // (id, name, reliability, status)
) -> String {
    if anchors.is_empty() {
        return escape_markdown("No anchors found.");
    }

    let title = escape_markdown("Anchors");
    let mut lines = vec![format!("*{title}*\n")];

    for (id, name, reliability, status) in anchors {
        let status_emoji = match status.as_str() {
            "green" => "\u{2705}",
            "yellow" => "\u{26A0}\u{FE0F}",
            _ => "\u{274C}",
        };

        lines.push(format!(
            "{status_emoji} *{name}*\n   ID: `{id}` \\| Reliability: {rel:.1}%",
            status_emoji = status_emoji,
            name = escape_markdown(name),
            id = escape_markdown(id),
            rel = reliability,
        ));
    }

    lines.join("\n")
}

pub fn format_anchor_detail(
    name: &str,
    stellar_account: &str,
    reliability: f64,
    total_txns: i64,
    successful_txns: i64,
    failed_txns: i64,
    status: &str,
) -> String {
    let title = escape_markdown(&format!("Anchor: {}", name));
    let status_emoji = match status {
        "green" => "\u{2705}",
        "yellow" => "\u{26A0}\u{FE0F}",
        _ => "\u{274C}",
    };

    format!(
        "*{title}* {se}\n\n\
         Account: `{acct}`\n\
         Reliability: {rel:.1}%\n\
         Total Transactions: {total}\n\
         Successful: {success}\n\
         Failed: {failed}\n\
         Status: {status}",
        title = title,
        se = status_emoji,
        acct = escape_markdown(stellar_account),
        rel = reliability,
        total = total_txns,
        success = successful_txns,
        failed = failed_txns,
        status = escape_markdown(status),
    )
}

pub fn format_help() -> String {
    let title = escape_markdown("Stellar Insights Bot");
    let cmds = [
        ("/status", "System health summary"),
        ("/corridors", "Top corridors with metrics"),
        ("/corridor <key>", "Detailed corridor info"),
        ("/anchors", "List anchors with reliability"),
        ("/anchor <id>", "Detailed anchor info"),
        ("/subscribe", "Subscribe to alerts"),
        ("/unsubscribe", "Unsubscribe from alerts"),
        ("/help", "Show this message"),
    ];

    let mut lines = vec![format!("*{title}*\n\nAvailable commands:\n")];
    for (cmd, desc) in &cmds {
        lines.push(format!(
            "`{cmd}` \\- {desc}",
            cmd = escape_markdown(cmd),
            desc = escape_markdown(desc),
        ));
    }

    lines.join("\n")
}
