use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ElkHealthResponse {
    pub status: String,
    pub elasticsearch: ComponentHealth,
    pub logstash: ComponentHealth,
    pub kibana: ComponentHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub reachable: bool,
    pub details: Option<serde_json::Value>,
}

pub async fn elk_health_check() -> impl IntoResponse {
    let elasticsearch_health = check_elasticsearch().await;
    let logstash_health = check_logstash().await;
    let kibana_health = check_kibana().await;

    let overall_status = if elasticsearch_health.reachable
        && logstash_health.reachable
        && kibana_health.reachable
    {
        "healthy"
    } else {
        "degraded"
    };

    let response = ElkHealthResponse {
        status: overall_status.to_string(),
        elasticsearch: elasticsearch_health,
        logstash: logstash_health,
        kibana: kibana_health,
    };

    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

async fn check_elasticsearch() -> ComponentHealth {
    let url = std::env::var("ELASTICSEARCH_URL").unwrap_or_else(|_| "http://localhost:9200".to_string());
    
    match reqwest::get(format!("{}/_cluster/health", url)).await {
        Ok(response) if response.status().is_success() => {
            let details = response.json::<serde_json::Value>().await.ok();
            ComponentHealth {
                status: details
                    .as_ref()
                    .and_then(|d| d.get("status"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                reachable: true,
                details,
            }
        }
        _ => ComponentHealth {
            status: "unreachable".to_string(),
            reachable: false,
            details: None,
        },
    }
}

async fn check_logstash() -> ComponentHealth {
    let url = std::env::var("LOGSTASH_URL").unwrap_or_else(|_| "http://localhost:9600".to_string());
    
    match reqwest::get(format!("{}/_node/stats", url)).await {
        Ok(response) if response.status().is_success() => {
            let details = response.json::<serde_json::Value>().await.ok();
            ComponentHealth {
                status: "running".to_string(),
                reachable: true,
                details,
            }
        }
        _ => ComponentHealth {
            status: "unreachable".to_string(),
            reachable: false,
            details: None,
        },
    }
}

async fn check_kibana() -> ComponentHealth {
    let url = std::env::var("KIBANA_URL").unwrap_or_else(|_| "http://localhost:5601".to_string());
    
    match reqwest::get(format!("{}/api/status", url)).await {
        Ok(response) if response.status().is_success() => {
            let details = response.json::<serde_json::Value>().await.ok();
            ComponentHealth {
                status: details
                    .as_ref()
                    .and_then(|d| d.get("status"))
                    .and_then(|s| s.get("overall"))
                    .and_then(|o| o.get("state"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                reachable: true,
                details,
            }
        }
        _ => ComponentHealth {
            status: "unreachable".to_string(),
            reachable: false,
            details: None,
        },
    }
}

#[derive(Debug, Serialize)]
pub struct LoggingMetrics {
    pub logs_sent: u64,
    pub logs_failed: u64,
    pub connection_status: String,
    pub last_error: Option<String>,
}

pub async fn logging_metrics() -> impl IntoResponse {
    // This would integrate with your actual metrics collection
    let metrics = LoggingMetrics {
        logs_sent: 0, // Replace with actual counter
        logs_failed: 0,
        connection_status: "connected".to_string(),
        last_error: None,
    };

    Json(metrics)
}
