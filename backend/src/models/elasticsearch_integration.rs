use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticsearchConfig {
    pub hosts: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub index_prefix: String,
    pub bulk_size: usize,
    pub timeout_secs: u64,
}

impl Default for ElasticsearchConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["http://localhost:9200".to_string()],
            username: None,
            password: None,
            index_prefix: "payraider".to_string(),
            bulk_size: 1000,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub filters: HashMap<String, String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub total: u64,
    pub results: Vec<SearchResult>,
    pub took_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetrics {
    pub total_indexed: u64,
    pub failed_indexes: u64,
    pub last_index_time: Option<chrono::DateTime<chrono::Utc>>,
}
