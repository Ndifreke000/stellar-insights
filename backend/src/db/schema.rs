/// Database schema definitions for the backend
pub struct Schema;

impl Schema {
    /// Create all tables if they don't exist
    /// This is a helper for manual verification or if migrations aren't used
    pub const CREATE_ANCHORS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS anchors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            stellar_account TEXT NOT NULL UNIQUE,
            home_domain TEXT,
            total_transactions INTEGER DEFAULT 0,
            successful_transactions INTEGER DEFAULT 0,
            failed_transactions INTEGER DEFAULT 0,
            total_volume_usd REAL DEFAULT 0,
            avg_settlement_time_ms INTEGER DEFAULT 0,
            reliability_score REAL DEFAULT 0,
            status TEXT DEFAULT 'green',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
    "#;

    pub const CREATE_ASSETS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            anchor_id TEXT NOT NULL REFERENCES anchors(id) ON DELETE CASCADE,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_supply REAL,
            num_holders INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(asset_code, asset_issuer)
        );
    "#;

    pub const CREATE_CORRIDORS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS corridors (
            id TEXT PRIMARY KEY,
            source_asset_code TEXT NOT NULL,
            source_asset_issuer TEXT NOT NULL,
            destination_asset_code TEXT NOT NULL,
            destination_asset_issuer TEXT NOT NULL,
            reliability_score REAL DEFAULT 0,
            status TEXT DEFAULT 'active',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
        );
    "#;

    pub const CREATE_METRICS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS metrics (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            value REAL NOT NULL,
            entity_id TEXT,
            entity_type TEXT,
            timestamp TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
    "#;

    pub const CREATE_SNAPSHOTS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            data TEXT NOT NULL,
            hash TEXT,
            epoch INTEGER,
            timestamp TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            verification_status TEXT DEFAULT 'pending',
            verified_at TEXT
        );
    "#;

    pub const CREATE_CONTRACT_EVENTS: &'static str = r#"
        CREATE TABLE IF NOT EXISTS contract_events (
            id TEXT PRIMARY KEY,
            contract_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            epoch INTEGER,
            hash TEXT,
            timestamp INTEGER,
            ledger INTEGER NOT NULL,
            transaction_hash TEXT NOT NULL,
            verification_status TEXT DEFAULT 'pending',
            verified_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
    "#;

    pub const CREATE_CONTRACT_EVENTS_INDEXES: &'static str = r#"
        CREATE INDEX IF NOT EXISTS idx_contract_events_created_at ON contract_events(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_contract_events_ledger ON contract_events(ledger DESC);
        CREATE INDEX IF NOT EXISTS idx_contract_events_epoch ON contract_events(epoch DESC);
        CREATE INDEX IF NOT EXISTS idx_contract_events_contract_id ON contract_events(contract_id);
        CREATE INDEX IF NOT EXISTS idx_contract_events_verification_status ON contract_events(verification_status);
        CREATE INDEX IF NOT EXISTS idx_contract_events_event_type ON contract_events(event_type);
    "#;
}
