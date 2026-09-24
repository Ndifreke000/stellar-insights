use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::field_selection_parameter::{
    FieldSelection, FieldSelectionConfig, FieldSelectionError,
};
use crate::models::network_context_middleware::NetworkContext;

/// Middleware that parses and validates the `fields` query parameter,
/// enabling clients (including mobile) to request only the fields they need.
#[derive(Clone)]
pub struct FieldSelectionParameter {
    config: FieldSelectionConfig,
    /// Tracks the total number of field-selection requests processed.
    request_count: Arc<RwLock<u64>>,
}

impl FieldSelectionParameter {
    pub fn new(config: FieldSelectionConfig) -> Self {
        Self {
            config,
            request_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Parse the raw `fields` query-parameter value in the context of the
    /// given network.  Returns the validated [`FieldSelection`] on success or
    /// a descriptive [`FieldSelectionError`] on failure.
    pub async fn process(
        &self,
        raw_fields: &str,
        context: &NetworkContext,
    ) -> Result<FieldSelection, FieldSelectionError> {
        let selection = FieldSelection::parse(raw_fields, self.config.max_fields)?;

        let mut count = self.request_count.write().await;
        *count += 1;

        tracing::info!(
            network = ?context.network,
            fields = ?selection.fields,
            total_requests = *count,
            "Field selection parameter processed"
        );

        Ok(selection)
    }

    /// Returns the total number of requests processed so far.
    pub async fn request_count(&self) -> u64 {
        *self.request_count.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_context_middleware::NetworkContext;

    fn middleware() -> FieldSelectionParameter {
        FieldSelectionParameter::new(FieldSelectionConfig::default())
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = FieldSelectionParameter::new(FieldSelectionConfig::default());
        let result = instance
            .process("id,name,status", &NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        let sel = result.unwrap();
        assert!(sel.includes("id"));
        assert!(sel.includes("name"));
        assert!(!sel.includes("unknown_field"));
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let result = middleware()
            .process("volume,success_rate", &NetworkContext::mainnet())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_field_list_returns_error() {
        let result = middleware().process("", &NetworkContext::testnet()).await;
        assert!(matches!(result, Err(FieldSelectionError::EmptyFieldList)));
    }

    #[tokio::test]
    async fn test_invalid_field_name_returns_error() {
        let result = middleware()
            .process("id,bad-field!", &NetworkContext::testnet())
            .await;
        assert!(matches!(
            result,
            Err(FieldSelectionError::InvalidFieldName(_))
        ));
    }

    #[tokio::test]
    async fn test_too_many_fields_returns_error() {
        let config = FieldSelectionConfig { max_fields: 2 };
        let instance = FieldSelectionParameter::new(config);
        let result = instance.process("a,b,c", &NetworkContext::testnet()).await;
        assert!(matches!(
            result,
            Err(FieldSelectionError::TooManyFields { .. })
        ));
    }

    #[tokio::test]
    async fn test_request_count_increments() {
        let instance = middleware();
        let ctx = NetworkContext::testnet();
        let _ = instance.process("id", &ctx).await;
        let _ = instance.process("name", &ctx).await;
        assert_eq!(instance.request_count().await, 2);
    }

    #[tokio::test]
    async fn test_whitespace_trimmed_from_fields() {
        let result = middleware()
            .process(" id , name ", &NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        let sel = result.unwrap();
        assert!(sel.includes("id"));
        assert!(sel.includes("name"));
    }

    #[tokio::test]
    async fn test_includes_empty_selection_matches_all() {
        let sel = FieldSelection::default();
        assert!(sel.includes("anything"));
    }
}
