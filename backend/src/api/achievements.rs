//! Achievements and quest definitions API
//! Serves challenge metadata for the gamification frontend and evaluates
//! quest/achievement progress server-side from activity events.
//!
//! Progress is never trusted from the client: callers submit the activity
//! events they observed, the server derives which quests are satisfied, and
//! returns a signed completion record the client can verify.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct QuestDef {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub xp: u32,
    pub path_match: Vec<String>,
    pub badge: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AchievementDef {
    pub id: String,
    pub title: String,
    pub description: String,
    pub badge: String,
    pub condition: String,
    pub threshold: u32,
}

fn quest_definitions() -> Vec<QuestDef> {
    vec![
        QuestDef {
            id: "view-corridor".to_string(),
            title: "Corridor Explorer".to_string(),
            description: "View the corridors page to understand asset flow between networks."
                .to_string(),
            category: "exploration".to_string(),
            xp: 50,
            path_match: vec!["/corridors".to_string()],
            badge: "🧭".to_string(),
        },
        QuestDef {
            id: "view-anchor".to_string(),
            title: "Anchor Analyst".to_string(),
            description: "Analyze an anchor's metrics and performance.".to_string(),
            category: "analysis".to_string(),
            xp: 75,
            path_match: vec!["/anchors".to_string()],
            badge: "⚓".to_string(),
        },
        QuestDef {
            id: "view-analytics".to_string(),
            title: "Data Explorer".to_string(),
            description: "Explore analytics dashboards and insights.".to_string(),
            category: "analysis".to_string(),
            xp: 50,
            path_match: vec!["/analytics".to_string()],
            badge: "📊".to_string(),
        },
        QuestDef {
            id: "view-trustlines".to_string(),
            title: "Trustline Scout".to_string(),
            description: "Discover trustline statistics across the network.".to_string(),
            category: "network".to_string(),
            xp: 50,
            path_match: vec!["/trustlines".to_string()],
            badge: "🔗".to_string(),
        },
        QuestDef {
            id: "view-health".to_string(),
            title: "Network Guardian".to_string(),
            description: "Check network health and status.".to_string(),
            category: "network".to_string(),
            xp: 50,
            path_match: vec!["/health".to_string()],
            badge: "💚".to_string(),
        },
        QuestDef {
            id: "view-liquidity-pools".to_string(),
            title: "DeFi Voyager".to_string(),
            description: "Explore liquidity pools and AMM data.".to_string(),
            category: "defi".to_string(),
            xp: 100,
            path_match: vec!["/liquidity-pools".to_string(), "/liquidity".to_string()],
            badge: "🌊".to_string(),
        },
        QuestDef {
            id: "view-dashboard".to_string(),
            title: "Terminal Access".to_string(),
            description: "Access the main dashboard.".to_string(),
            category: "exploration".to_string(),
            xp: 25,
            path_match: vec!["/dashboard".to_string(), "/".to_string()],
            badge: "🖥️".to_string(),
        },
        QuestDef {
            id: "view-sep6".to_string(),
            title: "SEP Specialist".to_string(),
            description: "Learn about Stellar Ecosystem Proposals (SEP-6).".to_string(),
            category: "education".to_string(),
            xp: 75,
            path_match: vec!["/sep6".to_string()],
            badge: "📚".to_string(),
        },
    ]
}

fn achievement_definitions() -> Vec<AchievementDef> {
    vec![
        AchievementDef {
            id: "first-quest".to_string(),
            title: "First Steps".to_string(),
            description: "Complete your first quest".to_string(),
            badge: "🌟".to_string(),
            condition: "quests".to_string(),
            threshold: 1,
        },
        AchievementDef {
            id: "explorer-3".to_string(),
            title: "Explorer".to_string(),
            description: "Complete 3 quests".to_string(),
            badge: "🗺️".to_string(),
            condition: "quests".to_string(),
            threshold: 3,
        },
        AchievementDef {
            id: "master-5".to_string(),
            title: "Quest Master".to_string(),
            description: "Complete 5 quests".to_string(),
            badge: "🏆".to_string(),
            condition: "quests".to_string(),
            threshold: 5,
        },
        AchievementDef {
            id: "champion".to_string(),
            title: "Champion".to_string(),
            description: "Complete all quests".to_string(),
            badge: "👑".to_string(),
            condition: "quests".to_string(),
            threshold: 8,
        },
    ]
}

/// A single activity event reported by the client. The server only uses the
/// `path` to match quest definitions; any client-supplied progress is ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivityEvent {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProgressRequest {
    pub account: String,
    #[serde(default)]
    pub events: Vec<ActivityEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletionRecord {
    pub account: String,
    pub quests: Vec<String>,
    pub achievements: Vec<String>,
    pub xp: u32,
    pub issued_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressResponse {
    pub record: CompletionRecord,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Server-authoritative evaluation of quest/achievement progress.
///
/// Pure function: given the set of observed activity paths, derive which
/// quests are satisfied and which achievements are unlocked. No client
/// progress values are consulted, so forged client progress has no effect.
pub fn evaluate_progress(observed_paths: &[String]) -> (Vec<String>, Vec<String>, u32) {
    let observed: HashSet<&str> = observed_paths.iter().map(String::as_str).collect();

    let mut completed_quests = Vec::new();
    let mut xp = 0u32;
    for quest in quest_definitions() {
        if quest
            .path_match
            .iter()
            .any(|p| observed.contains(p.as_str()))
        {
            xp += quest.xp;
            completed_quests.push(quest.id);
        }
    }

    let completed_count = completed_quests.len() as u32;
    let achievements = achievement_definitions()
        .into_iter()
        .filter(|a| a.condition == "quests" && completed_count >= a.threshold)
        .map(|a| a.id)
        .collect();

    (completed_quests, achievements, xp)
}

/// Sign a completion record with the server secret (HMAC-style keyed hash).
fn sign_record(secret: &str, account: &str, quests: &[String], achievements: &[String], xp: u32, issued_at: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"|");
    hasher.update(account.as_bytes());
    hasher.update(b"|");
    hasher.update(quests.join(",").as_bytes());
    hasher.update(b"|");
    hasher.update(achievements.join(",").as_bytes());
    hasher.update(b"|");
    hasher.update(xp.to_le_bytes());
    hasher.update(b"|");
    hasher.update(issued_at.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

/// Simple per-account sliding-window rate limiter for progress submissions.
struct RateLimiter {
    window: Duration,
    max_requests: usize,
    hits: Mutex<std::collections::HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    fn new(window: Duration, max_requests: usize) -> Self {
        Self {
            window,
            max_requests,
            hits: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut hits = match self.hits.lock() {
            Ok(h) => h,
            Err(_) => return false,
        };
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_requests {
            return false;
        }
        entry.push(now);
        true
    }
}

#[derive(Clone)]
pub struct AchievementsState {
    secret: String,
    limiter: std::sync::Arc<RateLimiter>,
}

impl AchievementsState {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            limiter: std::sync::Arc::new(RateLimiter::new(Duration::from_secs(60), 30)),
        }
    }
}

/// GET /api/achievements/quests - Get all available quests
#[utoipa::path(
    get,
    path = "/api/achievements/quests",
    responses(
        (status = 200, description = "List of quest definitions")
    ),
    tag = "Achievements"
)]
async fn get_quests() -> Json<Vec<QuestDef>> {
    Json(quest_definitions())
}

/// GET /api/achievements/achievements - Get all achievements
#[utoipa::path(
    get,
    path = "/api/achievements/achievements",
    responses(
        (status = 200, description = "List of achievement definitions")
    ),
    tag = "Achievements"
)]
async fn get_achievements() -> Json<Vec<AchievementDef>> {
    Json(achievement_definitions())
}

/// POST /api/achievements/progress - Evaluate progress server-side and return
/// a signed completion record. Client-supplied progress is ignored.
async fn post_progress(
    State(state): State<AchievementsState>,
    Json(req): Json<ProgressRequest>,
) -> Result<Json<ProgressResponse>, (StatusCode, Json<ApiError>)> {
    if req.account.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "account is required".to_string(),
            }),
        ));
    }

    if !state.limiter.check(&req.account) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                error: "rate limit exceeded".to_string(),
            }),
        ));
    }

    let paths: Vec<String> = req.events.into_iter().map(|e| e.path).collect();
    let (quests, achievements, xp) = evaluate_progress(&paths);
    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let signature = sign_record(&state.secret, &req.account, &quests, &achievements, xp, issued_at);

    Ok(Json(ProgressResponse {
        record: CompletionRecord {
            account: req.account,
            quests,
            achievements,
            xp,
            issued_at,
            signature,
        },
    }))
}

pub fn routes() -> Router {
    let secret = std::env::var("ACHIEVEMENTS_SIGNING_SECRET")
        .unwrap_or_else(|_| "stellar-insights-dev-secret".to_string());
    let state = AchievementsState::new(secret);
    Router::new()
        .route("/quests", get(get_quests))
        .route("/achievements", get(get_achievements))
        .route("/progress", post(post_progress))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forged_progress_has_no_effect() {
        // No observed activity -> nothing completed regardless of client claims.
        let (quests, achievements, xp) = evaluate_progress(&[]);
        assert!(quests.is_empty());
        assert!(achievements.is_empty());
        assert_eq!(xp, 0);
    }

    #[test]
    fn observed_paths_unlock_quests_and_achievements() {
        let paths = vec!["/corridors".to_string()];
        let (quests, achievements, xp) = evaluate_progress(&paths);
        assert_eq!(quests, vec!["view-corridor".to_string()]);
        assert_eq!(achievements, vec!["first-quest".to_string()]);
        assert_eq!(xp, 50);
    }

    #[test]
    fn unknown_paths_are_ignored() {
        let paths = vec!["/not-a-quest".to_string()];
        let (quests, achievements, xp) = evaluate_progress(&paths);
        assert!(quests.is_empty());
        assert!(achievements.is_empty());
        assert_eq!(xp, 0);
    }

    #[test]
    fn signature_is_deterministic_and_keyed() {
        let quests = vec!["view-corridor".to_string()];
        let a = sign_record("secret-a", "acct", &quests, &[], 50, 1);
        let b = sign_record("secret-a", "acct", &quests, &[], 50, 1);
        let c = sign_record("secret-b", "acct", &quests, &[], 50, 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn rate_limiter_blocks_after_max() {
        let limiter = RateLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.check("acct"));
        assert!(limiter.check("acct"));
        assert!(!limiter.check("acct"));
        assert!(limiter.check("other"));
    }
}
