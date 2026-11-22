//! Versioned, resumable storage migration framework.
//!
//! Migrations are expressed as a registry of steps, each moving storage from
//! `from_version` to `to_version`. A step processes at most `batch_size` items
//! per call and persists a cursor so it can resume across invocations, keeping
//! each transaction within resource limits. While a migration is in progress
//! the contract is paused for writes.

use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

/// Storage key for the current storage schema version.
const VERSION_KEY: Symbol = symbol_short!("ST_VER");
/// Storage key for the in-progress migration cursor.
const CURSOR_KEY: Symbol = symbol_short!("ST_CUR");
/// Storage key for the write-pause flag.
const PAUSED_KEY: Symbol = symbol_short!("ST_PAUSE");

/// Persisted cursor describing an in-flight migration step.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MigrationCursor {
    pub from_version: u32,
    pub to_version: u32,
    /// Number of items already processed by this step.
    pub processed: u32,
}

/// A single versioned migration step.
///
/// `apply` processes up to `batch_size` items starting at `cursor.processed`
/// and returns the number of items processed in this call. Returning `0`
/// signals the step is complete.
pub struct MigrationStep {
    pub from_version: u32,
    pub to_version: u32,
    pub apply: fn(&Env, u32, u32) -> u32,
}

/// Returns the registry of known migration steps.
///
/// Steps are ordered by `from_version`; each step advances exactly one version.
pub fn registry() -> Vec<MigrationStep> {
    // NOTE: steps are appended here as new schema versions are introduced.
    // Each entry must move storage from `from_version` to `from_version + 1`.
    Vec::new(&Env::default())
}

/// Returns the current storage schema version (0 if unset).
pub fn current_version(env: &Env) -> u32 {
    env.storage().instance().get(&VERSION_KEY).unwrap_or(0)
}

/// Returns `true` while a migration is in progress (writes are paused).
pub fn is_paused(env: &Env) -> bool {
    env.storage().instance().get(&PAUSED_KEY).unwrap_or(false)
}

/// Returns the active migration cursor, if any.
pub fn cursor(env: &Env) -> Option<MigrationCursor> {
    env.storage().instance().get(&CURSOR_KEY)
}

/// Looks up the step that migrates from `from_version`.
fn find_step(env: &Env, from_version: u32) -> Option<MigrationStep> {
    let steps = registry();
    let mut i = 0;
    while i < steps.len() {
        let step = steps.get(i).unwrap();
        if step.from_version == from_version {
            return Some(step);
        }
        i += 1;
    }
    let _ = env;
    None
}

/// Runs one bounded batch of the migration, resuming from the persisted cursor.
///
/// Returns `true` when all steps up to `target_version` have completed and the
/// contract has been unpaused, `false` when more calls are required.
pub fn run_batch(env: &Env, target_version: u32, batch_size: u32) -> bool {
    let mut version = current_version(env);

    if version >= target_version {
        // Nothing to do; ensure we are not left paused.
        env.storage().instance().set(&PAUSED_KEY, &false);
        env.storage().instance().remove(&CURSOR_KEY);
        return true;
    }

    // Pause writes for the duration of the migration.
    env.storage().instance().set(&PAUSED_KEY, &true);

    let mut active = match cursor(env) {
        Some(c) => c,
        None => MigrationCursor {
            from_version: version,
            to_version: version + 1,
            processed: 0,
        },
    };

    let step = match find_step(env, active.from_version) {
        Some(s) => s,
        None => {
            // No step registered for this version: cannot proceed safely.
            return false;
        }
    };

    let processed = (step.apply)(env, active.processed, batch_size);
    active.processed = active.processed.saturating_add(processed);

    if processed == 0 {
        // Step finished: advance the version and clear the cursor.
        version = step.to_version;
        env.storage().instance().set(&VERSION_KEY, &version);
        env.storage().instance().remove(&CURSOR_KEY);

        if version >= target_version {
            env.storage().instance().set(&PAUSED_KEY, &false);
            return true;
        }
        // More steps remain; they will start on the next call.
        return false;
    }

    // Persist the cursor so the next call resumes where we left off.
    env.storage().instance().set(&CURSOR_KEY, &active);
    false
}
