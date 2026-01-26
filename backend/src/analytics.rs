use crate::models::{AnchorMetrics, AnchorStatus};

pub mod corridor;

/// Performance metrics for an anchor's individual asset
#[derive(Debug, Clone)]
pub struct AnchorAssetPerformance {
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub total_volume_usd: f64,
}

/// Comprehensive reliability score for an anchor
#[derive(Debug, Clone)]
pub struct AnchorReliabilityScore {
    pub anchor_address: String,
    pub composite_score: f64, // 0-100 scale
    pub asset_performance_score: f64,
    pub volume_score: f64,
    pub asset_diversity_score: f64,
    pub total_assets: usize,
    pub total_volume_usd: f64,
    pub weighted_success_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Compute anchor reliability metrics based on transaction data
pub fn compute_anchor_metrics(
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    avg_settlement_time_ms: Option<i32>,
) -> AnchorMetrics {
    if total_transactions == 0 {
        return AnchorMetrics {
            success_rate: 0.0,
            failure_rate: 0.0,
            reliability_score: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            avg_settlement_time_ms: None,
            status: AnchorStatus::Red,
        };
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let failure_rate = (failed_transactions as f64 / total_transactions as f64) * 100.0;

    // Round to 2 decimal places for consistency
    let success_rate = (success_rate * 100.0).round() / 100.0;
    let failure_rate = (failure_rate * 100.0).round() / 100.0;

    // Compute reliability score (0-100)
    // Formula: (success_rate * 0.5) + (settlement_time_score * 0.25) + (volume_consistency * 0.25)
    // For MVP, we'll use a simplified formula focused on success rate and settlement time
    let settlement_time_score = calculate_settlement_time_score(avg_settlement_time_ms);
    let reliability_score = (success_rate * 0.7) + (settlement_time_score * 0.3);

    let status = AnchorStatus::from_metrics(success_rate, failure_rate);

    AnchorMetrics {
        success_rate,
        failure_rate,
        reliability_score,
        total_transactions,
        successful_transactions,
        failed_transactions,
        avg_settlement_time_ms,
        status,
    }
}

/// Calculate settlement time score (0-100)
/// Lower settlement time = higher score
fn calculate_settlement_time_score(avg_settlement_time_ms: Option<i32>) -> f64 {
    const MAX_SETTLEMENT_TIME_MS: f64 = 10000.0; // 10 seconds
    const MIN_SETTLEMENT_TIME_MS: f64 = 1000.0; // 1 second

    match avg_settlement_time_ms {
        Some(time_ms) if time_ms <= MIN_SETTLEMENT_TIME_MS as i32 => 100.0,
        Some(time_ms) if time_ms >= MAX_SETTLEMENT_TIME_MS as i32 => 0.0,
        Some(time_ms) => {
            let normalized = (MAX_SETTLEMENT_TIME_MS - time_ms as f64)
                / (MAX_SETTLEMENT_TIME_MS - MIN_SETTLEMENT_TIME_MS);
            normalized * 100.0
        }
        None => 50.0, // Default middle score if no data
    }
}

/// Calculate assets issued per anchor
pub fn count_assets_per_anchor(assets: &[String]) -> usize {
    assets.len()
}

/// Compute comprehensive anchor reliability score based on asset performance metrics
/// 
/// This function aggregates multiple dimensions of anchor performance:
/// - Asset performance (weighted success rate)
/// - Volume (logarithmically scaled)
/// - Asset diversity
/// 
/// # Arguments
/// * `asset_performances` - Slice of asset performance metrics for the anchor
/// * `network_max_volume` - Maximum volume across all anchors in the network for normalization
/// 
/// # Returns
/// `AnchorReliabilityScore` with composite score (0-100) and component scores
pub fn compute_anchor_reliability_score(
    asset_performances: &[AnchorAssetPerformance],
    network_max_volume: f64,
) -> AnchorReliabilityScore {
    // Handle empty asset list
    if asset_performances.is_empty() {
        return AnchorReliabilityScore {
            anchor_address: String::new(),
            composite_score: 0.0,
            asset_performance_score: 0.0,
            volume_score: 0.0,
            asset_diversity_score: 0.0,
            total_assets: 0,
            total_volume_usd: 0.0,
            weighted_success_rate: 0.0,
            timestamp: chrono::Utc::now(),
        };
    }

    // Calculate total volume and weighted success rate
    let mut total_volume_usd = 0.0;
    let mut weighted_success_sum = 0.0;

    for asset in asset_performances {
        total_volume_usd += asset.total_volume_usd;
        
        // Calculate success rate for this asset
        if asset.total_transactions > 0 {
            let success_rate = (asset.successful_transactions as f64 / asset.total_transactions as f64) * 100.0;
            weighted_success_sum += success_rate * asset.total_volume_usd;
        }
    }

    // 1. Calculate weighted_success_rate
    let weighted_success_rate = if total_volume_usd > 0.0 {
        weighted_success_sum / total_volume_usd
    } else {
        0.0
    };

    // 2. Calculate asset_performance_score (0-100)
    // Use weighted_success_rate directly as percentage
    let asset_performance_score = weighted_success_rate;

    // 3. Calculate volume_score (0-100)
    // Logarithmic scale to handle wide range of volumes
    let volume_score = if network_max_volume > 0.0 {
        let log_volume = (total_volume_usd + 1.0).log10();
        let log_max = (network_max_volume + 1.0).log10();
        (log_volume / log_max) * 100.0
    } else {
        // If network_max_volume is 0, use a simpler approach
        if total_volume_usd > 0.0 {
            50.0 // Default middle score if we have volume but no comparison
        } else {
            0.0
        }
    };

    // 4. Calculate asset_diversity_score (0-100)
    // Rewards anchors with up to 10 assets, caps at 100
    let total_assets = asset_performances.len();
    let asset_diversity_score = ((total_assets as f64 / 10.0).min(1.0)) * 100.0;

    // 5. Calculate composite_score
    // Weights: 60% performance, 30% volume, 10% diversity
    let composite_score = (0.6 * asset_performance_score) 
        + (0.3 * volume_score) 
        + (0.1 * asset_diversity_score);

    AnchorReliabilityScore {
        anchor_address: String::new(), // Caller will set this
        composite_score,
        asset_performance_score,
        volume_score,
        asset_diversity_score,
        total_assets,
        total_volume_usd,
        weighted_success_rate,
        timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_anchor_metrics_perfect_anchor() {
        let metrics = compute_anchor_metrics(1000, 995, 5, Some(2000));

        assert_eq!(metrics.total_transactions, 1000);
        assert_eq!(metrics.successful_transactions, 995);
        assert_eq!(metrics.failed_transactions, 5);
        assert_eq!(metrics.success_rate, 99.5);
        assert_eq!(metrics.failure_rate, 0.5);
        assert!(metrics.reliability_score > 90.0);
        assert_eq!(metrics.status, AnchorStatus::Green);
    }

    #[test]
    fn test_compute_anchor_metrics_yellow_anchor() {
        let metrics = compute_anchor_metrics(1000, 960, 40, Some(5000));

        assert_eq!(metrics.success_rate, 96.0);
        assert_eq!(metrics.failure_rate, 4.0);
        assert_eq!(metrics.status, AnchorStatus::Yellow);
    }

    #[test]
    fn test_compute_anchor_metrics_red_anchor() {
        let metrics = compute_anchor_metrics(1000, 900, 100, Some(9000));

        assert_eq!(metrics.success_rate, 90.0);
        assert_eq!(metrics.failure_rate, 10.0);
        assert_eq!(metrics.status, AnchorStatus::Red);
    }

    #[test]
    fn test_compute_anchor_metrics_no_transactions() {
        let metrics = compute_anchor_metrics(0, 0, 0, None);

        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.failure_rate, 0.0);
        assert_eq!(metrics.reliability_score, 0.0);
        assert_eq!(metrics.status, AnchorStatus::Red);
    }

    #[test]
    fn test_settlement_time_score_fast() {
        let score = calculate_settlement_time_score(Some(500));
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_settlement_time_score_slow() {
        let score = calculate_settlement_time_score(Some(12000));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_settlement_time_score_medium() {
        let score = calculate_settlement_time_score(Some(5000));
        assert!(score > 40.0 && score < 60.0);
    }

    #[test]
    fn test_count_assets() {
        let assets = vec!["USDC".to_string(), "EURC".to_string(), "BTC".to_string()];
        assert_eq!(count_assets_per_anchor(&assets), 3);
    }

    #[test]
    fn test_compute_anchor_reliability_score_empty_assets() {
        let score = compute_anchor_reliability_score(&[], 1000000.0);
        
        assert_eq!(score.composite_score, 0.0);
        assert_eq!(score.asset_performance_score, 0.0);
        assert_eq!(score.volume_score, 0.0);
        assert_eq!(score.asset_diversity_score, 0.0);
        assert_eq!(score.total_assets, 0);
        assert_eq!(score.total_volume_usd, 0.0);
        assert_eq!(score.weighted_success_rate, 0.0);
    }

    #[test]
    fn test_compute_anchor_reliability_score_perfect_performance() {
        let assets = vec![
            AnchorAssetPerformance {
                asset_code: "USDC".to_string(),
                asset_issuer: "ISSUER1".to_string(),
                total_transactions: 1000,
                successful_transactions: 1000,
                failed_transactions: 0,
                total_volume_usd: 100000.0,
            },
        ];
        
        let score = compute_anchor_reliability_score(&assets, 1000000.0);
        
        assert_eq!(score.weighted_success_rate, 100.0);
        assert_eq!(score.asset_performance_score, 100.0);
        assert_eq!(score.total_assets, 1);
        assert_eq!(score.total_volume_usd, 100000.0);
        assert!(score.composite_score > 50.0); // Should be decent with perfect performance
    }

    #[test]
    fn test_compute_anchor_reliability_score_weighted_success() {
        let assets = vec![
            AnchorAssetPerformance {
                asset_code: "USDC".to_string(),
                asset_issuer: "ISSUER1".to_string(),
                total_transactions: 100,
                successful_transactions: 100, // 100% success
                failed_transactions: 0,
                total_volume_usd: 80000.0, // 80% of volume
            },
            AnchorAssetPerformance {
                asset_code: "EURC".to_string(),
                asset_issuer: "ISSUER2".to_string(),
                total_transactions: 100,
                successful_transactions: 50, // 50% success
                failed_transactions: 50,
                total_volume_usd: 20000.0, // 20% of volume
            },
        ];
        
        let score = compute_anchor_reliability_score(&assets, 1000000.0);
        
        // Weighted: (100 * 80000 + 50 * 20000) / 100000 = 90
        assert_eq!(score.weighted_success_rate, 90.0);
        assert_eq!(score.asset_performance_score, 90.0);
        assert_eq!(score.total_assets, 2);
        assert_eq!(score.total_volume_usd, 100000.0);
        assert_eq!(score.asset_diversity_score, 20.0); // 2/10 * 100
    }

    #[test]
    fn test_compute_anchor_reliability_score_diversity() {
        let assets: Vec<AnchorAssetPerformance> = (0..15)
            .map(|i| AnchorAssetPerformance {
                asset_code: format!("ASSET{}", i),
                asset_issuer: "ISSUER".to_string(),
                total_transactions: 100,
                successful_transactions: 95,
                failed_transactions: 5,
                total_volume_usd: 10000.0,
            })
            .collect();
        
        let score = compute_anchor_reliability_score(&assets, 1000000.0);
        
        assert_eq!(score.total_assets, 15);
        assert_eq!(score.asset_diversity_score, 100.0); // Capped at 100
        assert_eq!(score.weighted_success_rate, 95.0);
    }

    #[test]
    fn test_compute_anchor_reliability_score_zero_network_volume() {
        let assets = vec![
            AnchorAssetPerformance {
                asset_code: "USDC".to_string(),
                asset_issuer: "ISSUER1".to_string(),
                total_transactions: 100,
                successful_transactions: 100,
                failed_transactions: 0,
                total_volume_usd: 50000.0,
            },
        ];
        
        let score = compute_anchor_reliability_score(&assets, 0.0);
        
        assert_eq!(score.volume_score, 50.0); // Default middle score
        assert!(score.composite_score > 0.0);
    }

    #[test]
    fn test_compute_anchor_reliability_score_composite_weights() {
        let assets = vec![
            AnchorAssetPerformance {
                asset_code: "USDC".to_string(),
                asset_issuer: "ISSUER1".to_string(),
                total_transactions: 100,
                successful_transactions: 100, // 100% success
                failed_transactions: 0,
                total_volume_usd: 1000000.0, // Max volume
            },
        ];
        
        let score = compute_anchor_reliability_score(&assets, 1000000.0);
        
        // Performance: 100, Volume: ~100, Diversity: 10
        // Composite: 0.6*100 + 0.3*100 + 0.1*10 = 60 + 30 + 1 = 91
        assert!(score.composite_score > 90.0 && score.composite_score < 92.0);
    }

    #[test]
    fn test_realistic_anchor_scenarios() {
        // Scenario 1: Established USDC-like anchor
        let usdc_anchor = vec![
            AnchorAssetPerformance {
                asset_code: "USDC".to_string(),
                asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
                total_transactions: 10000,
                successful_transactions: 9950,
                failed_transactions: 50,
                total_volume_usd: 50_000_000.0,
            },
        ];
        
        let usdc_score = compute_anchor_reliability_score(&usdc_anchor, 100_000_000.0);
        println!("\nEstablished Anchor (USDC-like):");
        println!("  Composite Score: {:.2}", usdc_score.composite_score);
        println!("  Performance: {:.2}", usdc_score.asset_performance_score);
        println!("  Volume: {:.2}", usdc_score.volume_score);
        println!("  Diversity: {:.2}", usdc_score.asset_diversity_score);
        
        // Scenario 2: New regional anchor
        let new_anchor = vec![
            AnchorAssetPerformance {
                asset_code: "NGN".to_string(),
                asset_issuer: "GBXXX".to_string(),
                total_transactions: 500,
                successful_transactions: 485,
                failed_transactions: 15,
                total_volume_usd: 500_000.0,
            },
        ];
        
        let new_score = compute_anchor_reliability_score(&new_anchor, 100_000_000.0);
        println!("\nNew Regional Anchor:");
        println!("  Composite Score: {:.2}", new_score.composite_score);
        println!("  Performance: {:.2}", new_score.asset_performance_score);
        println!("  Volume: {:.2}", new_score.volume_score);
        println!("  Diversity: {:.2}", new_score.asset_diversity_score);
        
        // Verify established anchor has higher score
        assert!(usdc_score.composite_score > new_score.composite_score);
        println!("\nEstablished anchor scores higher than new anchor");
    }
}
