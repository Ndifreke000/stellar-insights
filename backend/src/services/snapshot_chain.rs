//! Off-chain verifier for the payraider snapshot hash chain (#2782).
//!
//! Every stored epoch carries
//! `chain_hash = sha256(prev_chain_hash || epoch_be_u64 || snapshot_hash)`,
//! with an all-zero genesis. Given the records read from the contract (or from
//! `SNAP_SUB` events, which carry `chain_hash`), [`verify_chain`] recomputes
//! every link and reports the first epoch whose stored values do not match.
//! Checking the result against the contract's current `get_chain_head()` then
//! proves the whole history, so an admin-driven upgrade that rewrites any past
//! epoch is detected.
//!
//! Pure functions only, so it is testable without RPC or a database.

use sha2::{Digest, Sha256};

/// Genesis value of the chain.
pub const GENESIS: [u8; 32] = [0u8; 32];

/// One stored epoch as returned by `get_snapshot_record`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRecord {
    pub epoch: u64,
    pub hash: [u8; 32],
    pub prev_epoch: u64,
    pub prev_chain_hash: [u8; 32],
    pub chain_hash: [u8; 32],
}

/// Why a chain failed to verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// No records supplied.
    Empty,
    /// Records are not in strictly increasing epoch order.
    OutOfOrder { epoch: u64 },
    /// `chain_hash` does not recompute from the record's own fields.
    BadLink { epoch: u64 },
    /// The record does not point back at the record before it.
    BadBackLink { epoch: u64 },
    /// The recomputed tip differs from the expected head.
    HeadMismatch,
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainError::Empty => write!(f, "no chain records"),
            ChainError::OutOfOrder { epoch } => write!(f, "epoch {epoch} out of order"),
            ChainError::BadLink { epoch } => write!(f, "epoch {epoch}: chain hash mismatch"),
            ChainError::BadBackLink { epoch } => {
                write!(f, "epoch {epoch}: does not link to previous epoch")
            }
            ChainError::HeadMismatch => write!(f, "chain tip differs from on-chain head"),
        }
    }
}

impl std::error::Error for ChainError {}

/// `sha256(prev || epoch_be || hash)` — must match `payraider::chain_link`.
pub fn chain_link(prev: &[u8; 32], epoch: u64, hash: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(prev);
    hasher.update(epoch.to_be_bytes());
    hasher.update(hash);
    let out = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&out[..]);
    bytes
}

/// Verify a contiguous run of chain records (ascending epochs) and return the
/// tip's chain hash. The first record's back-link is trusted as given, so a
/// partial range can be verified; pass the full history from genesis, or
/// check `records[0].prev_chain_hash` against a trusted value, to anchor it.
pub fn verify_chain(records: &[ChainRecord]) -> Result<[u8; 32], ChainError> {
    if records.is_empty() {
        return Err(ChainError::Empty);
    }
    let mut prev: Option<&ChainRecord> = None;
    for rec in records {
        if let Some(p) = prev {
            if rec.epoch <= p.epoch {
                return Err(ChainError::OutOfOrder { epoch: rec.epoch });
            }
            if rec.prev_epoch != p.epoch || rec.prev_chain_hash != p.chain_hash {
                return Err(ChainError::BadBackLink { epoch: rec.epoch });
            }
        }
        if chain_link(&rec.prev_chain_hash, rec.epoch, &rec.hash) != rec.chain_hash {
            return Err(ChainError::BadLink { epoch: rec.epoch });
        }
        prev = Some(rec);
    }
    Ok(prev.map(|p| p.chain_hash).unwrap_or(GENESIS))
}

/// [`verify_chain`] from genesis, additionally requiring the tip to equal
/// the contract's current `get_chain_head()`.
pub fn verify_full_history(
    records: &[ChainRecord],
    on_chain_head: &[u8; 32],
) -> Result<(), ChainError> {
    let first = records.first().ok_or(ChainError::Empty)?;
    if first.prev_epoch != 0 || first.prev_chain_hash != GENESIS {
        return Err(ChainError::BadBackLink { epoch: first.epoch });
    }
    let tip = verify_chain(records)?;
    if &tip != on_chain_head {
        return Err(ChainError::HeadMismatch);
    }
    Ok(())
}

/// Build a valid chain from `(epoch, hash)` pairs — what the contract does on
/// each `submit_snapshot`. Used by tests and by tooling that replays events.
pub fn build_chain(items: &[(u64, [u8; 32])]) -> Vec<ChainRecord> {
    let mut out = Vec::with_capacity(items.len());
    let mut prev_epoch = 0u64;
    let mut prev_chain = GENESIS;
    for (epoch, hash) in items {
        let chain_hash = chain_link(&prev_chain, *epoch, hash);
        out.push(ChainRecord {
            epoch: *epoch,
            hash: *hash,
            prev_epoch,
            prev_chain_hash: prev_chain,
            chain_hash,
        });
        prev_epoch = *epoch;
        prev_chain = chain_hash;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u8) -> [u8; 32] {
        [n; 32]
    }

    fn sample() -> Vec<ChainRecord> {
        build_chain(&[(1, h(1)), (2, h(2)), (5, h(5)), (9, h(9))])
    }

    #[test]
    fn valid_chain_verifies_to_head() {
        let chain = sample();
        let head = chain.last().unwrap().chain_hash;
        assert_eq!(verify_chain(&chain), Ok(head));
        assert_eq!(verify_full_history(&chain, &head), Ok(()));
    }

    #[test]
    fn first_link_is_hash_of_first_epoch_over_genesis() {
        let chain = sample();
        assert_eq!(chain[0].chain_hash, chain_link(&GENESIS, 1, &h(1)));
    }

    #[test]
    fn rewritten_snapshot_hash_is_detected() {
        let mut chain = sample();
        chain[2].hash = h(77);
        assert_eq!(verify_chain(&chain), Err(ChainError::BadLink { epoch: 5 }));
    }

    #[test]
    fn self_consistent_rewrite_breaks_the_next_back_link() {
        let mut chain = sample();
        chain[1].hash = h(88);
        chain[1].chain_hash = chain_link(&chain[1].prev_chain_hash, 2, &chain[1].hash);
        assert_eq!(
            verify_chain(&chain),
            Err(ChainError::BadBackLink { epoch: 5 })
        );
    }

    #[test]
    fn rewriting_the_whole_suffix_is_caught_by_the_head() {
        let honest = sample();
        let head = honest.last().unwrap().chain_hash;
        let forged = build_chain(&[(1, h(1)), (2, h(2)), (5, h(50)), (9, h(9))]);
        assert!(verify_chain(&forged).is_ok());
        assert_eq!(
            verify_full_history(&forged, &head),
            Err(ChainError::HeadMismatch)
        );
    }

    #[test]
    fn dropped_epoch_and_reordering_are_detected() {
        let mut chain = sample();
        chain.remove(1);
        assert_eq!(
            verify_chain(&chain),
            Err(ChainError::BadBackLink { epoch: 5 })
        );
        let mut chain = sample();
        chain.swap(1, 2);
        assert_eq!(verify_chain(&chain), Err(ChainError::OutOfOrder { epoch: 2 }));
    }

    #[test]
    fn empty_and_non_genesis_start_are_rejected() {
        assert_eq!(verify_chain(&[]), Err(ChainError::Empty));
        let chain = sample();
        let head = chain.last().unwrap().chain_hash;
        assert_eq!(
            verify_full_history(&chain[1..], &head),
            Err(ChainError::BadBackLink { epoch: 2 })
        );
    }
}
