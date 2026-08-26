#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::super::resolvers::*;
    use super::super::schema::*;
    use async_graphql::*;
    use std::sync::Arc;

    // ── Type Construction Tests ────────────────────────────────────────────────

    #[test]
    fn test_anchor_type_fields() {
        let anchor = AnchorType {
            id: "test-id".to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H".to_string(),
            home_domain: Some("example.com".to_string()),
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            total_volume_usd: 50000.0,
            avg_settlement_time_ms: 2000,
            reliability_score: 95.0,
            status: "green".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(anchor.id, "test-id");
        assert_eq!(anchor.name, "Test Anchor");
        assert_eq!(anchor.reliability_score, 95.0);
        assert_eq!(anchor.status, "green");
        assert!(anchor.home_domain.is_some());
    }

    #[test]
    fn test_corridor_type_fields() {
        let corridor = CorridorType {
            id: "test-corridor".to_string(),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H".to_string(),
            destination_asset_code: "EUR".to_string(),
            destination_asset_issuer: "GARE5K4KJL3VQ4E5VZ6JZ7X7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q".to_string(),
            reliability_score: 98.5,
            status: "active".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(corridor.source_asset_code, "USDC");
        assert_eq!(corridor.destination_asset_code, "EUR");
        assert_eq!(corridor.reliability_score, 98.5);
    }

    #[test]
    fn test_payment_type_fields() {
        let payment = PaymentType {
            id: "pay-1".to_string(),
            transaction_hash: "abc123".to_string(),
            source_account: "GADDR1".to_string(),
            destination_account: "GADDR2".to_string(),
            asset_type: "native".to_string(),
            asset_code: Some("XLM".to_string()),
            asset_issuer: None,
            source_asset_code: "XLM".to_string(),
            source_asset_issuer: "".to_string(),
            destination_asset_code: "USDC".to_string(),
            destination_asset_issuer: "GADDR3".to_string(),
            amount: 100.50,
            successful: true,
            timestamp: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
        };

        assert_eq!(payment.amount, 100.50);
        assert!(payment.successful);
    }

    #[test]
    fn test_liquidity_pool_type_fields() {
        let pool = LiquidityPoolType {
            pool_id: "pool-1".to_string(),
            pool_type: "constant_product".to_string(),
            fee_bp: 30,
            total_trustlines: 150,
            total_shares: "1000000".to_string(),
            reserve_a_asset_code: "XLM".to_string(),
            reserve_a_asset_issuer: None,
            reserve_a_amount: 50000.0,
            reserve_b_asset_code: "USDC".to_string(),
            reserve_b_asset_issuer: Some("GADDR".to_string()),
            reserve_b_amount: 25000.0,
            total_value_usd: 75000.0,
            volume_24h_usd: 10000.0,
            fees_earned_24h_usd: 30.0,
            apy: 12.5,
            impermanent_loss_pct: 0.5,
            trade_count_24h: 200,
            last_synced_at: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(pool.pool_id, "pool-1");
        assert_eq!(pool.apy, 12.5);
        assert_eq!(pool.fee_bp, 30);
    }

    #[test]
    fn test_trustline_stat_type_fields() {
        let stat = TrustlineStatType {
            asset_code: "USDC".to_string(),
            asset_issuer: "GADDR".to_string(),
            total_trustlines: 5000,
            authorized_trustlines: 4800,
            unauthorized_trustlines: 200,
            total_supply: 1000000.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(stat.asset_code, "USDC");
        assert_eq!(stat.total_trustlines, 5000);
    }

    #[test]
    fn test_snapshot_type_fields() {
        let snapshot = SnapshotType {
            id: "snap-1".to_string(),
            entity_id: "anchor-1".to_string(),
            entity_type: "anchor".to_string(),
            data: "{}".to_string(),
            hash: Some("abc123".to_string()),
            epoch: Some(42),
            timestamp: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };

        assert_eq!(snapshot.entity_type, "anchor");
        assert_eq!(snapshot.epoch, Some(42));
    }

    // ── Connection Type Tests ──────────────────────────────────────────────────

    #[test]
    fn test_anchors_connection() {
        let conn = AnchorsConnection {
            nodes: vec![],
            total_count: 0,
            has_next_page: false,
        };

        assert!(conn.nodes.is_empty());
        assert!(!conn.has_next_page);
    }

    #[test]
    fn test_payments_connection_pagination() {
        let conn = PaymentsConnection {
            nodes: vec![],
            total_count: 25,
            has_next_page: true,
        };

        assert_eq!(conn.total_count, 25);
        assert!(conn.has_next_page);
    }

    // ── Filter Type Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_anchor_filter_construction() {
        let filter = AnchorFilter {
            status: Some("green".to_string()),
            min_reliability_score: Some(90.0),
            search: Some("example".to_string()),
        };

        assert_eq!(filter.status, Some("green".to_string()));
        assert_eq!(filter.min_reliability_score, Some(90.0));
        assert!(filter.search.is_some());
    }

    #[test]
    fn test_corridor_filter_construction() {
        let filter = CorridorFilter {
            source_asset_code: Some("USDC".to_string()),
            destination_asset_code: None,
            status: Some("active".to_string()),
            min_reliability_score: Some(95.0),
        };

        assert_eq!(filter.source_asset_code, Some("USDC".to_string()));
        assert!(filter.destination_asset_code.is_none());
    }

    #[test]
    fn test_payment_filter_construction() {
        let filter = PaymentFilter {
            source_account: Some("GADDR1".to_string()),
            destination_account: None,
            asset_code: Some("USDC".to_string()),
            successful: Some(true),
            min_amount: Some(10.0),
            max_amount: Some(1000.0),
        };

        assert_eq!(filter.asset_code, Some("USDC".to_string()));
        assert_eq!(filter.min_amount, Some(10.0));
    }

    #[test]
    fn test_liquidity_pool_filter_construction() {
        let filter = LiquidityPoolFilter {
            asset_a_code: Some("XLM".to_string()),
            asset_b_code: Some("USDC".to_string()),
            min_apy: Some(5.0),
            min_total_value_usd: Some(10000.0),
        };

        assert_eq!(filter.asset_a_code, Some("XLM".to_string()));
        assert_eq!(filter.min_apy, Some(5.0));
    }

    #[test]
    fn test_trustline_filter_construction() {
        let filter = TrustlineFilter {
            asset_code: Some("USDC".to_string()),
            min_total_trustlines: Some(1000),
        };

        assert_eq!(filter.asset_code, Some("USDC".to_string()));
        assert_eq!(filter.min_total_trustlines, Some(1000));
    }

    #[test]
    fn test_time_range_input() {
        let time_range = TimeRangeInput {
            start: chrono::Utc::now(),
            end: chrono::Utc::now(),
        };

        assert!(time_range.start <= time_range.end);
    }

    #[test]
    fn test_pagination_input() {
        let pagination = PaginationInput {
            limit: Some(20),
            offset: Some(10),
        };

        assert_eq!(pagination.limit, Some(20));
        assert_eq!(pagination.offset, Some(10));
    }

    // ── Mutation Input Tests ───────────────────────────────────────────────────

    #[test]
    fn test_create_anchor_input() {
        let input = CreateAnchorInput {
            name: "Test Anchor".to_string(),
            stellar_account: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H".to_string(),
            home_domain: Some("example.com".to_string()),
        };

        assert_eq!(input.name, "Test Anchor");
        assert_eq!(input.stellar_account.len(), 56);
    }

    #[test]
    fn test_create_corridor_input() {
        let input = CreateCorridorInput {
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H".to_string(),
            destination_asset_code: "EUR".to_string(),
            destination_asset_issuer: "GARE5K4KJL3VQ4E5VZ6JZ7X7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q".to_string(),
        };

        assert_eq!(input.source_asset_code, "USDC");
        assert_eq!(input.destination_asset_code, "EUR");
    }

    #[test]
    fn test_update_anchor_metrics_input() {
        let input = UpdateAnchorMetricsInput {
            anchor_id: "anchor-1".to_string(),
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            avg_settlement_time_ms: Some(2000),
            volume_usd: Some(50000.0),
        };

        assert_eq!(input.total_transactions, 1000);
        assert_eq!(input.successful_transactions + input.failed_transactions, input.total_transactions);
    }

    // ── Mutation Payload Tests ─────────────────────────────────────────────────

    #[test]
    fn test_create_anchor_payload() {
        let payload = CreateAnchorPayload {
            anchor: AnchorType {
                id: "test".to_string(),
                name: "Test".to_string(),
                stellar_account: "GADDR".to_string(),
                home_domain: None,
                total_transactions: 0,
                successful_transactions: 0,
                failed_transactions: 0,
                total_volume_usd: 0.0,
                avg_settlement_time_ms: 0,
                reliability_score: 0.0,
                status: "green".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            success: true,
            message: "Created".to_string(),
        };

        assert!(payload.success);
        assert_eq!(payload.message, "Created");
    }

    #[test]
    fn test_create_corridor_payload() {
        let payload = CreateCorridorPayload {
            corridor: CorridorType {
                id: "test".to_string(),
                source_asset_code: "USDC".to_string(),
                source_asset_issuer: "GADDR1".to_string(),
                destination_asset_code: "EUR".to_string(),
                destination_asset_issuer: "GADDR2".to_string(),
                reliability_score: 0.0,
                status: "active".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            success: true,
            message: "Created".to_string(),
        };

        assert!(payload.success);
    }

    // ── Subscription Event Tests ───────────────────────────────────────────────

    #[test]
    fn test_corridor_update_event() {
        let event = CorridorUpdateEvent {
            corridor_key: "USDC-GADDR1-EUR-GADDR2".to_string(),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "GADDR1".to_string(),
            destination_asset_code: "EUR".to_string(),
            destination_asset_issuer: "GADDR2".to_string(),
            success_rate: Some(98.5),
            health_score: Some(95.0),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        };

        assert_eq!(event.corridor_key, "USDC-GADDR1-EUR-GADDR2");
        assert_eq!(event.success_rate, Some(98.5));
    }

    #[test]
    fn test_anchor_update_event() {
        let event = AnchorUpdateEvent {
            anchor_id: "anchor-1".to_string(),
            name: "Test Anchor".to_string(),
            reliability_score: 95.0,
            status: "green".to_string(),
        };

        assert_eq!(event.anchor_id, "anchor-1");
        assert_eq!(event.status, "green");
    }

    #[test]
    fn test_snapshot_update_event() {
        let event = SnapshotUpdateEvent {
            snapshot_id: "snap-1".to_string(),
            epoch: 42,
            timestamp: chrono::Utc::now().to_rfc3339(),
            hash: "abc123".to_string(),
        };

        assert_eq!(event.epoch, 42);
    }

    #[test]
    fn test_health_alert_event() {
        let event = HealthAlertEvent {
            corridor_id: "corridor-1".to_string(),
            severity: "critical".to_string(),
            message: "Reliability dropped below threshold".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        assert_eq!(event.severity, "critical");
    }

    #[test]
    fn test_new_payment_event() {
        let event = NewPaymentEvent {
            corridor_id: "corridor-1".to_string(),
            amount: 100.50,
            successful: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        assert_eq!(event.amount, 100.50);
        assert!(event.successful);
    }

    // ── Health Type Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_health_type() {
        let health = HealthType {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            database: "ok".to_string(),
            cache: "ok".to_string(),
            active_connections: 0,
        };

        assert_eq!(health.status, "ok");
        assert_eq!(health.version, "0.1.0");
    }

    // ── Search Results Tests ───────────────────────────────────────────────────

    #[test]
    fn test_search_results_empty() {
        let results = SearchResults {
            anchors: vec![],
            corridors: vec![],
            payments: vec![],
        };

        assert!(results.anchors.is_empty());
        assert!(results.corridors.is_empty());
        assert!(results.payments.is_empty());
    }

    // ── Stats Type Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_liquidity_pool_stats_type() {
        let stats = LiquidityPoolStatsType {
            total_pools: 100,
            total_liquidity_usd: 1000000.0,
            avg_pool_size_usd: 10000.0,
            total_value_locked_usd: 900000.0,
            total_volume_24h_usd: 500000.0,
            total_fees_24h_usd: 1500.0,
            avg_apy: 12.5,
            avg_impermanent_loss: 0.5,
        };

        assert_eq!(stats.total_pools, 100);
        assert_eq!(stats.avg_apy, 12.5);
    }

    #[test]
    fn test_trustline_metrics_type() {
        let metrics = TrustlineMetricsType {
            total_assets_tracked: 50,
            total_trustlines_across_network: 100000,
            active_assets: 30,
        };

        assert_eq!(metrics.total_assets_tracked, 50);
        assert_eq!(metrics.active_assets, 30);
    }

    // ── Anchor Metrics History Tests ───────────────────────────────────────────

    #[test]
    fn test_anchor_metrics_history_type() {
        let history = AnchorMetricsHistoryType {
            id: "history-1".to_string(),
            anchor_id: "anchor-1".to_string(),
            timestamp: chrono::Utc::now(),
            success_rate: 98.5,
            failure_rate: 1.5,
            reliability_score: 98.5,
            total_transactions: 1000,
            successful_transactions: 985,
            failed_transactions: 15,
            avg_settlement_time_ms: Some(2000),
            volume_usd: Some(50000.0),
            created_at: chrono::Utc::now(),
        };

        assert_eq!(history.success_rate + history.failure_rate, 100.0);
    }

    // ── Schema Construction Tests ──────────────────────────────────────────────

    #[test]
    fn test_schema_type_is_constructible() {
        // Verify the schema type alias resolves correctly
        let _: fn(Arc<sqlx::SqlitePool>, tokio::sync::broadcast::Sender<String>) -> AppSchema =
            build_schema;
    }

    // ── Liquidity Pool Snapshot Tests ──────────────────────────────────────────

    #[test]
    fn test_liquidity_pool_snapshot_type() {
        let snapshot = LiquidityPoolSnapshotType {
            id: 1,
            pool_id: "pool-1".to_string(),
            reserve_a_amount: 50000.0,
            reserve_b_amount: 25000.0,
            total_value_usd: 75000.0,
            volume_usd: 10000.0,
            fees_usd: 30.0,
            apy: 12.5,
            impermanent_loss_pct: 0.5,
            trade_count: 200,
            snapshot_at: chrono::Utc::now(),
        };

        assert_eq!(snapshot.id, 1);
        assert_eq!(snapshot.pool_id, "pool-1");
    }

    // ── Trustline Snapshot Tests ───────────────────────────────────────────────

    #[test]
    fn test_trustline_snapshot_type() {
        let snapshot = TrustlineSnapshotType {
            id: 1,
            asset_code: "USDC".to_string(),
            asset_issuer: "GADDR".to_string(),
            total_trustlines: 5000,
            authorized_trustlines: 4800,
            unauthorized_trustlines: 200,
            total_supply: 1000000.0,
            snapshot_at: chrono::Utc::now(),
        };

        assert_eq!(snapshot.id, 1);
        assert_eq!(snapshot.total_trustlines, 5000);
    }
}
