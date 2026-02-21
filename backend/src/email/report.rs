use serde::Serialize;

#[derive(Serialize)]
pub struct CorridorSummary {
    pub id: String,
    pub success_rate: f64,
    pub volume_usd: f64,
    pub avg_latency_ms: f64,
    pub change_pct: f64,
}

#[derive(Serialize)]
pub struct AnchorSummary {
    pub name: String,
    pub success_rate: f64,
    pub total_transactions: i64,
    pub volume_usd: f64,
}

#[derive(Serialize)]
pub struct DigestReport {
    pub period: String,
    pub top_corridors: Vec<CorridorSummary>,
    pub top_anchors: Vec<AnchorSummary>,
    pub total_volume: f64,
    pub avg_success_rate: f64,
}

pub fn generate_html_report(report: &DigestReport) -> String {
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        h1 {{ color: #333; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 12px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; }}
        .metric {{ font-size: 24px; font-weight: bold; color: #4CAF50; }}
        .positive {{ color: green; }}
        .negative {{ color: red; }}
    </style>
</head>
<body>
    <h1>Stellar Insights - {} Performance Report</h1>
    
    <h2>Overview</h2>
    <p>Total Volume: <span class="metric">${:.2}</span></p>
    <p>Average Success Rate: <span class="metric">{:.1}%</span></p>
    
    <h2>Top Corridors</h2>
    <table>
        <tr>
            <th>Corridor</th>
            <th>Success Rate</th>
            <th>Volume (USD)</th>
            <th>Avg Latency</th>
            <th>Change</th>
        </tr>
        {}
    </table>
    
    <h2>Top Anchors</h2>
    <table>
        <tr>
            <th>Anchor</th>
            <th>Success Rate</th>
            <th>Transactions</th>
            <th>Volume (USD)</th>
        </tr>
        {}
    </table>
</body>
</html>
"#,
        report.period,
        report.total_volume,
        report.avg_success_rate,
        report.top_corridors.iter().map(|c| format!(
            "<tr><td>{}</td><td>{:.1}%</td><td>${:.2}</td><td>{:.0}ms</td><td class='{}'>{:+.1}%</td></tr>",
            c.id, c.success_rate, c.volume_usd, c.avg_latency_ms,
            if c.change_pct >= 0.0 { "positive" } else { "negative" },
            c.change_pct
        )).collect::<Vec<_>>().join("\n"),
        report.top_anchors.iter().map(|a| format!(
            "<tr><td>{}</td><td>{:.1}%</td><td>{}</td><td>${:.2}</td></tr>",
            a.name, a.success_rate, a.total_transactions, a.volume_usd
        )).collect::<Vec<_>>().join("\n")
    )
}
