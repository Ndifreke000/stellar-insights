//! Horizon REST API models.

use serde::{Deserialize, Serialize};

// Asset Models (Horizon API)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonAsset {
    pub asset_type: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub num_claimable_balances: i32,
    pub num_liquidity_pools: i32,
    pub num_contracts: i32,
    pub accounts: AssetAccounts,
    pub claimable_balances_amount: String,
    pub liquidity_pools_amount: String,
    pub contracts_amount: String,
    pub balances: AssetBalances,
    pub flags: AssetFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAccounts {
    pub authorized: i32,
    pub authorized_to_maintain_liabilities: i32,
    pub unauthorized: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalances {
    pub authorized: String,
    pub authorized_to_maintain_liabilities: String,
    pub unauthorized: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFlags {
    pub auth_required: bool,
    pub auth_revocable: bool,
    pub auth_immutable: bool,
    pub auth_clawback_enabled: bool,
}

/// Represents a single asset balance change from the new Horizon API format.
///
/// The new Horizon response for Soroban-compatible payments includes an
/// `asset_balance_changes` array instead of top-level destination / amount /
/// `asset_code` fields.  Each entry describes one leg of a transfer.
///
/// Example JSON:
/// ```json
/// {
///   "asset_type": "credit_alphanum4",
///   "asset_code": "USDC",
///   "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
///   "type": "transfer",
///   "from": "GXXXXXXX...",
///   "to": "GDYYYYYY...",
///   "amount": "100.0000000"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalanceChange {
    /// The Stellar asset type (native, `credit_alphanum4`, `credit_alphanum12`)
    pub asset_type: String,
    /// Asset code – `None` for native XLM
    pub asset_code: Option<String>,
    /// Asset issuer – `None` for native XLM
    pub asset_issuer: Option<String>,
    /// The kind of balance change (e.g. "transfer")
    #[serde(rename = "type")]
    pub change_type: String,
    /// Source account of this balance change
    pub from: Option<String>,
    /// Destination account of this balance change
    pub to: Option<String>,
    /// Amount transferred in stroops-string format (e.g. "100.0000000")
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub paging_token: String,
    pub transaction_hash: String,
    pub source_account: String,
    #[serde(default)]
    pub destination: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
    pub created_at: String,
    // Path payment fields
    #[serde(rename = "type")]
    pub operation_type: Option<String>,
    // Source asset for path payments
    pub source_asset_type: Option<String>,
    pub source_asset_code: Option<String>,
    pub source_asset_issuer: Option<String>,
    pub source_amount: Option<String>,
    // For regular payments, 'from' field
    pub from: Option<String>,
    // For regular payments, 'to' field
    pub to: Option<String>,
    /// New Horizon API format: Soroban-compatible asset balance changes.
    /// When present the traditional top-level fields may be empty; callers
    /// should use the `get_*` helper methods which transparently check both.
    #[serde(default)]
    pub asset_balance_changes: Option<Vec<AssetBalanceChange>>,
    /// Whether the enclosing transaction succeeded. Horizon only returns
    /// failed operations when asked (`include_failed=true`); `None` means the
    /// source did not say, which is treated as successful.
    #[serde(default)]
    pub transaction_successful: Option<bool>,
}

impl Payment {
    /// False only when Horizon reported the transaction as failed.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.transaction_successful != Some(false)
    }
}

impl Payment {
    /// Returns the destination account, checking the new `asset_balance_changes`
    /// format first, then falling back to the legacy `destination` / `to` fields.
    #[must_use]
    pub fn get_destination(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                if let Some(ref to) = change.to {
                    return Some(to.clone());
                }
            }
        }
        if !self.destination.is_empty() {
            return Some(self.destination.clone());
        }
        self.to.clone()
    }

    /// Returns the transfer amount, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_amount(&self) -> String {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.amount.clone();
            }
        }
        self.amount.clone()
    }

    /// Returns the asset code, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_asset_code(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.asset_code.clone();
            }
        }
        self.asset_code.clone()
    }

    /// Returns the asset issuer, preferring `asset_balance_changes`.
    #[must_use]
    pub fn get_asset_issuer(&self) -> Option<String> {
        if let Some(ref changes) = self.asset_balance_changes {
            if let Some(change) = changes.first() {
                return change.asset_issuer.clone();
            }
        }
        self.asset_issuer.clone()
    }
}

// Horizon API Response Structures
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonOperation {
    pub id: String,
    pub paging_token: String,
    pub transaction_hash: String,
    pub source_account: String,
    #[serde(rename = "type")]
    pub operation_type: String,
    pub created_at: String,
    pub account: Option<String>,
    pub into: Option<String>,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonEffect {
    pub id: String,
    #[serde(rename = "type")]
    pub effect_type: String,
    pub account: Option<String>,
    pub amount: Option<String>,
    pub asset_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonTransaction {
    pub id: String,
    pub hash: String,
    pub ledger: u64,
    pub created_at: String,
    pub source_account: String,
    #[serde(rename = "fee_account")]
    pub fee_account: Option<String>,
    #[serde(rename = "fee_charged")]
    pub fee_charged: Option<String>,
    #[serde(rename = "max_fee")]
    pub max_fee: Option<String>,
    pub operation_count: u32,
    pub successful: bool,
    pub paging_token: String,
    #[serde(rename = "fee_bump_transaction")]
    pub fee_bump_transaction: Option<FeeBumpTransactionInfo>,
    #[serde(rename = "inner_transaction")]
    pub inner_transaction: Option<InnerTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBumpTransactionInfo {
    pub hash: String,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerTransaction {
    pub hash: String,
    #[serde(rename = "max_fee")]
    pub max_fee: Option<String>,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub ledger_close_time: String,
    pub base_account: String,
    pub base_amount: String,
    pub base_asset_type: String,
    pub base_asset_code: Option<String>,
    pub base_asset_issuer: Option<String>,
    pub counter_account: String,
    pub counter_amount: String,
    pub counter_asset_type: String,
    pub counter_asset_code: Option<String>,
    pub counter_asset_issuer: Option<String>,
    pub price: Price,
    pub trade_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub n: i64,
    pub d: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
    pub base: Asset,
    pub counter: Asset,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub price: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.asset_type == "native" {
            write!(f, "XLM")
        } else {
            write!(
                f,
                "{}:{}",
                self.asset_code.as_deref().unwrap_or(&self.asset_type),
                self.asset_issuer.as_deref().unwrap_or("")
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonResponse<T> {
    #[serde(rename = "_embedded")]
    pub embedded: Option<EmbeddedRecords<T>>,
    #[serde(flatten)]
    pub data: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedRecords<T> {
    pub records: Vec<T>,
}

// ============================================================================
// Path Finding Models (Horizon /paths/strict-send)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPath {
    pub source_asset_type: String,
    pub source_asset_code: Option<String>,
    pub source_asset_issuer: Option<String>,
    pub source_amount: String,
    pub destination_asset_type: String,
    pub destination_asset_code: Option<String>,
    pub destination_asset_issuer: Option<String>,
    pub destination_amount: String,
    pub path: Vec<HorizonPathEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPathEntry {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: String,
    pub price: String,
    pub price_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPathsResponse {
    pub _embedded: EmbeddedRecords<HorizonPath>,
}

// ============================================================================
// Liquidity Pool Models (Horizon API)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonPoolReserve {
    pub asset: String, // "native" or "CODE:ISSUER"
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonLiquidityPool {
    pub id: String,
    #[serde(rename = "fee_bp")]
    pub fee_bp: u32,
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(rename = "total_trustlines")]
    pub total_trustlines: u64,
    #[serde(rename = "total_shares")]
    pub total_shares: String,
    pub reserves: Vec<HorizonPoolReserve>,
    pub paging_token: Option<String>,
}
