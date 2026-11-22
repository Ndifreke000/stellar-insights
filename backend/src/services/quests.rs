//! Server-authoritative quest/achievement evaluation.
//!
//! Progress is derived exclusively from on-chain/activity events recorded by the
//! backend. Client-supplied progress is never trusted: callers pass the set of
//! observed events and the service recomputes progress from scratch. Completion
//! records are signed so clients can verify them without trusting the transport.

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::ApiError;

type HmacSha256 = Hmac<Sha256>;

/// A single on-chain/activity event observed by the backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ActivityEvent {
    /// Stable identifier of the account the event belongs to.
    pub account: String,
    /// Event kind, e.g. `swap`, `bridge`, `liquidity_add`.
    pub kind: String,
    /// Ledger sequence the event was observed at.
    pub ledger: u64,
}

/// Definition of a quest and the events that satisfy it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct QuestDefinition {
    pub id: String,
    /// Event kind that advances this quest.
    pub event_kind: String,
    /// Number of matching events required to complete the quest.
    pub required: u32,
}

/// Server-computed progress for a quest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct QuestProgress {
    pub quest_id: String,
    pub account: String,
    pub completed: u32,
    pub required: u32,
    pub is_complete: bool,
}

/// Signed completion record returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SignedCompletion {
    pub quest_id: String,
    pub account: String,
    pub completed: u32,
    pub required: u32,
    /// Hex-encoded HMAC-SHA256 over the canonical record payload.
    pub signature: String,
}

/// Service that evaluates quest progress from observed events.
#[derive(Clone)]
pub struct QuestService {
    definitions: Vec<QuestDefinition>,
    signing_key: Vec<u8>,
}

impl QuestService {
    pub fn new(definitions: Vec<QuestDefinition>, signing_key: impl Into<Vec<u8>>) -> Self {
        Self {
            definitions,
            signing_key: signing_key.into(),
        }
    }

    /// Recompute progress for `account` from the observed events.
    ///
    /// Only events belonging to `account` and matching a quest's `event_kind`
    /// count. Duplicate events (same account/kind/ledger) are counted once.
    pub fn evaluate(
        &self,
        account: &str,
        events: &[ActivityEvent],
    ) -> Result<Vec<QuestProgress>, ApiError> {
        if account.trim().is_empty() {
            return Err(ApiError::BadRequest("account must not be empty".into()));
        }

        let mut seen: HashMap<(&str, &str, u64), ()> = HashMap::new();
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for event in events {
            if event.account != account {
                continue;
            }
            let key = (event.account.as_str(), event.kind.as_str(), event.ledger);
            if seen.insert(key, ()).is_some() {
                continue;
            }
            *counts.entry(event.kind.as_str()).or_insert(0) += 1;
        }

        let progress = self
            .definitions
            .iter()
            .map(|def| {
                let completed = counts.get(def.event_kind.as_str()).copied().unwrap_or(0);
                QuestProgress {
                    quest_id: def.id.clone(),
                    account: account.to_string(),
                    completed,
                    required: def.required,
                    is_complete: completed >= def.required,
                }
            })
            .collect();

        Ok(progress)
    }

    /// Produce signed completion records for every completed quest.
    pub fn signed_completions(
        &self,
        progress: &[QuestProgress],
    ) -> Result<Vec<SignedCompletion>, ApiError> {
        progress
            .iter()
            .filter(|p| p.is_complete)
            .map(|p| {
                let signature = self.sign(p)?;
                Ok(SignedCompletion {
                    quest_id: p.quest_id.clone(),
                    account: p.account.clone(),
                    completed: p.completed,
                    required: p.required,
                    signature,
                })
            })
            .collect()
    }

    fn sign(&self, progress: &QuestProgress) -> Result<String, ApiError> {
        let payload = format!(
            "{}:{}:{}:{}",
            progress.quest_id, progress.account, progress.completed, progress.required
        );
        let mut mac = HmacSha256::new_from_slice(&self.signing_key)
            .map_err(|_| ApiError::Internal("invalid quest signing key".into()))?;
        mac.update(payload.as_bytes());
        Ok(hex_encode(&mac.finalize().into_bytes()))
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> QuestService {
        QuestService::new(
            vec![
                QuestDefinition {
                    id: "first-swap".into(),
                    event_kind: "swap".into(),
                    required: 1,
                },
                QuestDefinition {
                    id: "bridge-3".into(),
                    event_kind: "bridge".into(),
                    required: 3,
                },
            ],
            b"test-key".to_vec(),
        )
    }

    fn event(account: &str, kind: &str, ledger: u64) -> ActivityEvent {
        ActivityEvent {
            account: account.into(),
            kind: kind.into(),
            ledger,
        }
    }

    #[test]
    fn evaluates_progress_from_events() {
        let events = vec![
            event("alice", "swap", 1),
            event("alice", "bridge", 2),
            event("alice", "bridge", 3),
        ];
        let progress = service().evaluate("alice", &events).unwrap();
        let swap = progress.iter().find(|p| p.quest_id == "first-swap").unwrap();
        assert!(swap.is_complete);
        let bridge = progress.iter().find(|p| p.quest_id == "bridge-3").unwrap();
        assert_eq!(bridge.completed, 2);
        assert!(!bridge.is_complete);
    }

    #[test]
    fn ignores_other_accounts_and_duplicates() {
        let events = vec![
            event("alice", "swap", 1),
            event("alice", "swap", 1),
            event("bob", "swap", 2),
        ];
        let progress = service().evaluate("alice", &events).unwrap();
        let swap = progress.iter().find(|p| p.quest_id == "first-swap").unwrap();
        assert_eq!(swap.completed, 1);
    }

    #[test]
    fn rejects_empty_account() {
        assert!(service().evaluate("  ", &[]).is_err());
    }

    #[test]
    fn signs_only_completed_quests() {
        let events = vec![event("alice", "swap", 1)];
        let progress = service().evaluate("alice", &events).unwrap();
        let signed = service().signed_completions(&progress).unwrap();
        assert_eq!(signed.len(), 1);
        assert_eq!(signed[0].quest_id, "first-swap");
        assert!(!signed[0].signature.is_empty());
    }
}
