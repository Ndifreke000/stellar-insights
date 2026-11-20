//! Cost model for expensive operations.
//!
//! This module defines the cost calculation logic for export and analytics
//! endpoints, enabling cost-based rate limiting instead of simple request
//! counting.
//!
//! Cost units are debited from the client's budget; if insufficient, the
//! request is rejected with 429 Too Many Requests.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cost unit configuration.
/// A cost unit is an arbitrary measure - for example, 1 unit could represent
/// 1 corridor × 1 day of history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Cost per corridor in an export query.
    pub per_corridor: f64,
    /// Cost per day of date range in an export query.
    pub per_day: f64,
    /// Cost per analytics dashboard request.
    pub per_analytics_request: f64,
    /// Default budget for anonymous clients.
    pub default_anonymous_budget: f64,
    /// Default budget for authenticated clients.
    pub default_authenticated_budget: f64,
    /// Default budget for premium clients.
    pub default_premium_budget: f64,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            per_corridor: 10.0,
            per_day: 1.0,
            per_analytics_request: 5.0,
            default_anonymous_budget: 100.0,
            default_authenticated_budget: 1000.0,
            default_premium_budget: 10_000.0,
        }
    }
}

impl CostConfig {
    /// Calculate cost for a corridor export query.
    pub fn export_corridor_cost(&self, date_range_days: i64, corridor_count: usize) -> f64 {
        let range_cost = self.per_day * (date_range_days as f64);
        let corridor_cost = self.per_corridor * (corridor_count as f64);
        range_cost + corridor_cost
    }

    /// Calculate cost for a corridor export with single corridor.
    pub fn export_corridor_single_cost(&self, date_range_days: i64) -> f64 {
        self.export_corridor_cost(date_range_days, 1)
    }

    /// Calculate cost for a corridor export with all corridors.
    pub fn export_corridor_all_cost(&self, date_range_days: i64) -> f64 {
        // When no corridor_id is specified, estimate worst-case: all active corridors
        // Use a reasonable upper bound (e.g., 1000 corridors) to prevent unbounded costs
        self.export_corridor_cost(date_range_days, 1000)
    }
}

/// Client tier for budget allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientTier {
    Anonymous,
    Authenticated,
    Premium,
}

impl ClientTier {
    /// Get the budget multiplier for this tier.
    pub fn budget_multiplier(&self) -> f64 {
        match self {
            Self::Anonymous => 1.0,
            Self::Authenticated => 10.0,
            Self::Premium => 100.0,
        }
    }

    /// Get the budget for this tier from config.
    pub fn budget(&self, config: &CostConfig) -> f64 {
        match self {
            Self::Anonymous => config.default_anonymous_budget,
            Self::Authenticated => config.default_authenticated_budget,
            Self::Premium => config.default_premium_budget,
        }
    }
}

/// Cost tracking state per client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCostState {
    /// Current cost unit balance.
    pub balance: f64,
    /// When the balance was last updated.
    pub last_updated: i64,
    /// Tier of the client.
    pub tier: ClientTier,
}

impl ClientCostState {
    /// Create a new state with the given balance and tier.
    pub fn new(balance: f64, tier: ClientTier) -> Self {
        Self {
            balance,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            tier,
        }
    }

    /// Try to debit cost units. Returns Ok(true) if successful, Ok(false) if insufficient funds.
    pub fn try_debit(&mut self, cost: f64) -> Result<bool, &'static str> {
        if cost > self.balance {
            return Ok(false);
        }
        self.balance -= cost;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(true)
    }

    /// Credit cost units (for budget refresh).
    pub fn credit(&mut self, amount: f64) {
        self.balance += amount;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }

    /// Check if client has sufficient balance for the given cost.
    pub fn has_sufficient_balance(&self, cost: f64) -> bool {
        self.balance >= cost
    }
}

/// Error for cost-related operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostError {
    pub message: String,
    pub retry_after_seconds: u64,
}

impl std::fmt::Display for CostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CostError {}

/// Headers to return with cost information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostHeaders {
    /// Estimated cost of the request.
    pub cost_estimate: f64,
    /// Remaining budget after this request.
    pub remaining_budget: f64,
    /// Total budget for this client.
    pub total_budget: f64,
    /// Tier of the client.
    pub tier: String,
    /// When the budget refreshes (Unix timestamp).
    pub budget_refresh_at: i64,
}

impl CostHeaders {
    /// Create headers from cost state and estimated cost.
    pub fn from_state(state: &ClientCostState, cost_estimate: f64) -> Self {
        Self {
            cost_estimate,
            remaining_budget: state.balance - cost_estimate,
            total_budget: state.budget(&CostConfig::default()),
            tier: format!("{:?}", state.tier),
            budget_refresh_at: state.last_updated + (24 * 3600), // Daily refresh
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_cost_calculation() {
        let config = CostConfig::default();

        // 1 corridor × 1 day = 10 + 1 = 11 cost units
        let cost = config.export_corridor_single_cost(1);
        assert!((cost - 11.0).abs() < f64::EPSILON);

        // 1 corridor × 30 days = 10 + 30 = 40 cost units
        let cost = config.export_corridor_single_cost(30);
        assert!((cost - 40.0).abs() < f64::EPSILON);

        // 1 corridor × 365 days = 10 + 365 = 375 cost units
        let cost = config.export_corridor_single_cost(365);
        assert!((cost - 375.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_export_cost_all_corridors() {
        let config = CostConfig::default();
        // 1000 corridors × 1 day = 10000 + 1 = 10001 cost units
        let cost = config.export_corridor_all_cost(1);
        assert!((cost - 10001.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_budget_by_tier() {
        let config = CostConfig::default();

        assert_eq!(
            ClientTier::Anonymous.budget(&config),
            config.default_anonymous_budget
        );
        assert_eq!(
            ClientTier::Authenticated.budget(&config),
            config.default_authenticated_budget
        );
        assert_eq!(
            ClientTier::Premium.budget(&config),
            config.default_premium_budget
        );
    }

    #[test]
    fn test_budget_multiplier() {
        assert_eq!(ClientTier::Anonymous.budget_multiplier(), 1.0);
        assert_eq!(ClientTier::Authenticated.budget_multiplier(), 10.0);
        assert_eq!(ClientTier::Premium.budget_multiplier(), 100.0);
    }

    #[test]
    fn test_debit_succeeds() {
        let mut state = ClientCostState::new(100.0, ClientTier::Anonymous);
        assert!(state.try_debit(50.0).is_ok());
        assert!((state.balance - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_debit_fails_insufficient() {
        let mut state = ClientCostState::new(100.0, ClientTier::Anonymous);
        assert!(!state.try_debit(150.0).unwrap());
        assert!((state.balance - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_has_sufficient_balance() {
        let state = ClientCostState::new(100.0, ClientTier::Anonymous);
        assert!(state.has_sufficient_balance(50.0));
        assert!(!state.has_sufficient_balance(150.0));
    }

    #[test]
    fn test_credit() {
        let mut state = ClientCostState::new(100.0, ClientTier::Anonymous);
        state.credit(50.0);
        assert!((state.balance - 150.0).abs() < f64::EPSILON);
    }
}
