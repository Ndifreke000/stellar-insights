use soroban_sdk::{contracttype, Address, Env};

/// Number of ledgers to extend the TTL by on each persistent storage write.
/// At ~5s per ledger this is roughly 30 days.
pub const LEDGERS_TO_EXTEND: u32 = 518_400;

/// Threshold (in ledgers) below which the TTL is refreshed on access/write.
/// At ~5s per ledger this is roughly 7 days.
pub const LEDGERS_TO_EXTEND_THRESHOLD: u32 = 120_960;

#[contracttype]
#[derive(Clone)]
pub struct StorageKey {
    pub owner: Address,
    pub id: u32,
}

/// Persist a value and extend its TTL so it does not expire unexpectedly.
pub fn set_persistent<T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &StorageKey,
    value: &T,
) {
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, LEDGERS_TO_EXTEND_THRESHOLD, LEDGERS_TO_EXTEND);
}

/// Read a persistent value, refreshing its TTL when it is still present.
pub fn get_persistent<T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &StorageKey,
) -> Option<T> {
    let value = env.storage().persistent().get(key);
    if value.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(key, LEDGERS_TO_EXTEND_THRESHOLD, LEDGERS_TO_EXTEND);
    }
    value
}

/// Remove a persistent value from storage.
pub fn remove_persistent(env: &Env, key: &StorageKey) {
    env.storage().persistent().remove(key);
}

// ---------------------------------------------------------------------------
// Versioned storage migration framework
// ---------------------------------------------------------------------------

/// Storage key holding the current schema version of the contract.
#[contracttype]
#[derive(Clone)]
pub enum MigrationKey {
    /// Current applied schema version.
    Version,
    /// Cursor for the in-flight migration step (resumable batches).
    Cursor,
    /// Whether a migration is currently in progress (writes paused).
    InProgress,
}

/// A single versioned migration step.
///
/// Each step advances storage from `from_version` to `to_version` and processes
/// at most `batch_size` items per invocation so it stays within per-transaction
/// resource limits. `run` returns the next cursor; `None` signals completion.
pub struct MigrationStep {
    pub from_version: u32,
    pub to_version: u32,
    pub batch_size: u32,
    pub run: fn(&Env, u32) -> Option<u32>,
}

/// Registry of all known migration steps, ordered by `from_version`.
///
/// New steps are appended here as the schema evolves. The registry is the single
/// source of truth for the upgrade path.
pub fn migration_registry() -> &'static [MigrationStep] {
    &[
        // v0 -> v1: initial schema normalisation (no-op placeholder step).
        MigrationStep {
            from_version: 0,
            to_version: 1,
            batch_size: 50,
            run: |_env, cursor| {
                // v0 had no versioned keys; nothing to rewrite in bounded batches.
                let _ = cursor;
                None
            },
        },
        // v1 -> v2: reserved for future schema changes.
        MigrationStep {
            from_version: 1,
            to_version: 2,
            batch_size: 50,
            run: |_env, cursor| {
                let _ = cursor;
                None
            },
        },
    ]
}

/// Read the current schema version (defaults to 0 for pre-versioned storage).
pub fn get_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&MigrationKey::Version)
        .unwrap_or(0)
}

/// Whether a migration is currently in progress (writes must be paused).
pub fn is_migration_in_progress(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&MigrationKey::InProgress)
        .unwrap_or(false)
}

/// Guard used by write paths: panics while a migration is in progress.
pub fn assert_writes_allowed(env: &Env) {
    if is_migration_in_progress(env) {
        panic!("contract paused: migration in progress");
    }
}

/// Advance the migration by one bounded batch.
///
/// Resumable across calls: the cursor is persisted after each batch so a
/// subsequent call continues where the previous one stopped. Returns `true`
/// when the whole upgrade path has completed, `false` when more batches remain.
pub fn migrate(env: &Env) -> bool {
    let mut version = get_version(env);
    let registry = migration_registry();

    // Find the next applicable step for the current version.
    let step = match registry.iter().find(|s| s.from_version == version) {
        Some(step) => step,
        None => {
            // Nothing left to do; ensure we are not stuck paused.
            env.storage()
                .instance()
                .set(&MigrationKey::InProgress, &false);
            return true;
        }
    };

    // Mark migration in progress so writes are paused until completion.
    env.storage()
        .instance()
        .set(&MigrationKey::InProgress, &true);

    // Resume from the persisted cursor (0 when starting a fresh step).
    let cursor: u32 = env
        .storage()
        .instance()
        .get(&MigrationKey::Cursor)
        .unwrap_or(0);

    match (step.run)(env, cursor) {
        Some(next_cursor) => {
            // Persist cursor and stay paused; caller must invoke again.
            env.storage()
                .instance()
                .set(&MigrationKey::Cursor, &next_cursor);
            false
        }
        None => {
            // Step complete: bump version, reset cursor, continue next call.
            version = step.to_version;
            env.storage()
                .instance()
                .set(&MigrationKey::Version, &version);
            env.storage().instance().remove(&MigrationKey::Cursor);

            // If another step follows, keep paused; otherwise resume writes.
            let has_next = registry.iter().any(|s| s.from_version == version);
            if !has_next {
                env.storage()
                    .instance()
                    .set(&MigrationKey::InProgress, &false);
            }
            !has_next
        }
    }
}
