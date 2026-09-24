use crate::models::elasticsearch_integration::{
    ElasticsearchConfig, IndexMetrics, SearchQuery, SearchResponse, SearchResult,
};
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct ElasticsearchIntegration {
    config: ElasticsearchConfig,
    client: reqwest::Client,
    metrics: Arc<RwLock<IndexMetrics>>,
}

impl ElasticsearchIntegration {
    pub fn new(config: ElasticsearchConfig) -> Self {
        let client = reqwest::Client::new();
        let metrics = Arc::new(RwLock::new(IndexMetrics {
            total_indexed: 0,
            failed_indexes: 0,
            last_index_time: None,
        }));

        info!(
            "Elasticsearch integration initialized with hosts: {:?}",
            config.hosts
        );

        Self {
            config,
            client,
            metrics,
        }
    }

    pub async fn execute(&self) -> Result<serde_json::Value> {
        debug!("Executing Elasticsearch integration health check");
        self.health_check().await
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let url = format!("{}/_cluster/health", self.config.hosts[0]);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if response.status().is_success() {
            let health = response.json::<serde_json::Value>().await?;
            info!("Elasticsearch cluster health check passed");
            Ok(health)
        } else {
            error!(
                "Elasticsearch health check failed with status: {}",
                response.status()
            );
            Err(anyhow::anyhow!(
                "Elasticsearch health check failed: {}",
                response.status()
            ))
        }
    }

    pub async fn index_document(
        &self,
        index: &str,
        doc_id: &str,
        document: serde_json::Value,
    ) -> Result<()> {
        let url = format!(
            "{}/_doc/{}",
            self.get_index_url(index),
            urlencoding::encode(doc_id)
        );

        let response = self
            .client
            .put(&url)
            .json(&document)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if response.status().is_success() {
            let mut metrics = self.metrics.write().await;
            metrics.total_indexed += 1;
            metrics.last_index_time = Some(chrono::Utc::now());
            debug!("Document indexed successfully: {}", doc_id);
            Ok(())
        } else {
            let mut metrics = self.metrics.write().await;
            metrics.failed_indexes += 1;
            error!("Failed to index document {}: {}", doc_id, response.status());
            Err(anyhow::anyhow!(
                "Failed to index document: {}",
                response.status()
            ))
        }
    }

    pub async fn search(&self, index: &str, query: SearchQuery) -> Result<SearchResponse> {
        let start = Instant::now();
        let url = format!("{}/_search", self.get_index_url(index));

        let es_query = self.build_query(&query);
        let body = json!({
            "query": es_query,
            "from": query.offset,
            "size": query.limit,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if response.status().is_success() {
            let result = response.json::<serde_json::Value>().await?;
            let took_ms = start.elapsed().as_millis() as u64;

            let total = result["hits"]["total"]["value"].as_u64().unwrap_or(0);

            let results = result["hits"]["hits"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|hit| SearchResult {
                    id: hit["_id"].as_str().unwrap_or("").to_string(),
                    score: hit["_score"].as_f64().unwrap_or(0.0),
                    data: hit["_source"].clone(),
                })
                .collect();

            debug!("Search completed in {}ms, found {} results", took_ms, total);

            Ok(SearchResponse {
                total,
                results,
                took_ms,
            })
        } else {
            error!("Search failed with status: {}", response.status());
            Err(anyhow::anyhow!("Search failed: {}", response.status()))
        }
    }

    pub async fn delete_document(&self, index: &str, doc_id: &str) -> Result<()> {
        let url = format!(
            "{}/_doc/{}",
            self.get_index_url(index),
            urlencoding::encode(doc_id)
        );

        let response = self
            .client
            .delete(&url)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Document deleted successfully: {}", doc_id);
            Ok(())
        } else {
            error!(
                "Failed to delete document {}: {}",
                doc_id,
                response.status()
            );
            Err(anyhow::anyhow!(
                "Failed to delete document: {}",
                response.status()
            ))
        }
    }

    pub async fn get_metrics(&self) -> IndexMetrics {
        self.metrics.read().await.clone()
    }

    fn get_index_url(&self, index: &str) -> String {
        format!(
            "{}/{}-{}",
            self.config.hosts[0], self.config.index_prefix, index
        )
    }

    fn build_query(&self, query: &SearchQuery) -> serde_json::Value {
        let mut must_clauses = vec![json!({
            "multi_match": {
                "query": query.query,
                "fields": ["*"]
            }
        })];

        for (key, value) in &query.filters {
            must_clauses.push(json!({
                "term": {
                    key: value
                }
            }));
        }

        json!({
            "bool": {
                "must": must_clauses
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elasticsearch_config_default() {
        let config = ElasticsearchConfig::default();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.bulk_size, 1000);
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery {
            query: "test".to_string(),
            filters: Default::default(),
            limit: 10,
            offset: 0,
        };
        assert_eq!(query.query, "test");
        assert_eq!(query.limit, 10);
    }

    #[tokio::test]
    async fn test_elasticsearch_integration_creation() {
        let config = ElasticsearchConfig::default();
        let es = ElasticsearchIntegration::new(config);
        let metrics = es.get_metrics().await;
        assert_eq!(metrics.total_indexed, 0);
        assert_eq!(metrics.failed_indexes, 0);
    }
}
