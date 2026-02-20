// =========================
// Transactions domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingTransaction {
    pub id: String,
    pub source_account: String,
    pub xdr: String,
    pub required_signatures: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Signature {
    pub id: String,
    pub transaction_id: String,
    pub signer: String,
    pub signature: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransactionWithSignatures {
    #[serde(flatten)]
    pub transaction: PendingTransaction,
    pub collected_signatures: Vec<Signature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub hash: String,
    pub status: String,
}

// =========================
// Trustline domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrustlineStat {
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_trustlines: i64,
    pub authorized_trustlines: i64,
    pub unauthorized_trustlines: i64,
    pub total_supply: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrustlineSnapshot {
    pub id: i64,
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_trustlines: i64,
    pub authorized_trustlines: i64,
    pub unauthorized_trustlines: i64,
    pub total_supply: f64,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustlineMetrics {
    pub total_assets_tracked: i64,
    pub total_trustlines_across_network: i64,
    pub active_assets: i64,
}