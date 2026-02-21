//! Achievements and quest definitions API
//! Serves challenge metadata for the gamification frontend

use axum::{routing::get, Json, Router};
use serde::Serialize;

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

async fn get_quests() -> Json<Vec<QuestDef>> {
    Json(quest_definitions())
}

async fn get_achievements() -> Json<Vec<AchievementDef>> {
    Json(achievement_definitions())
}

pub fn routes() -> Router {
    Router::new()
        .route("/quests", get(get_quests))
        .route("/achievements", get(get_achievements))
}
