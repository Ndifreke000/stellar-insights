use async_graphql::Schema;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::resolvers::{MutationRoot, QueryRoot};
use super::subscription::SubscriptionRoot;

/// The consolidated application schema type.
///
/// Uses the database-backed `QueryRoot` and `MutationRoot` for full CRUD
/// operations, and `SubscriptionRoot` for real-time WebSocket subscriptions.
pub type AppSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Query-cost guards applied to the schema (Issue #2642).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphqlLimits {
    pub max_depth: usize,
    pub max_complexity: usize,
    pub introspection: bool,
}

impl GraphqlLimits {
    pub const DEFAULT_MAX_DEPTH: usize = 10;
    pub const DEFAULT_MAX_COMPLEXITY: usize = 500;

    /// Read `GRAPHQL_MAX_DEPTH`, `GRAPHQL_MAX_COMPLEXITY` and
    /// `GRAPHQL_INTROSPECTION`. Introspection defaults to off when
    /// `RUST_ENV`/`ENVIRONMENT` is `production`/`prod`.
    #[must_use]
    pub fn from_env() -> Self {
        let env_mode = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .ok();
        let parse = |key: &str, default: usize| {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };
        Self {
            max_depth: parse("GRAPHQL_MAX_DEPTH", Self::DEFAULT_MAX_DEPTH),
            max_complexity: parse("GRAPHQL_MAX_COMPLEXITY", Self::DEFAULT_MAX_COMPLEXITY),
            introspection: introspection_enabled(
                env_mode.as_deref(),
                std::env::var("GRAPHQL_INTROSPECTION").ok().as_deref(),
            ),
        }
    }
}

/// An explicit `GRAPHQL_INTROSPECTION` wins; otherwise introspection is on
/// everywhere except production.
#[must_use]
pub fn introspection_enabled(env_mode: Option<&str>, explicit: Option<&str>) -> bool {
    if let Some(value) = explicit {
        return matches!(value.to_lowercase().as_str(), "1" | "true" | "on" | "yes");
    }
    !matches!(
        env_mode.map(str::to_lowercase).as_deref(),
        Some("production" | "prod")
    )
}

/// Build the consolidated GraphQL schema with database access.
///
/// This is the primary schema builder used in production. It includes:
/// - Query resolvers for all entities (anchors, corridors, payments, etc.)
/// - Mutation resolvers for create/update/delete operations
/// - Subscription resolvers for real-time updates via WebSocket
///
/// Depth/complexity limits and the introspection toggle come from the env.
pub fn build_schema(pool: Arc<SqlitePool>, broadcast_tx: broadcast::Sender<String>) -> AppSchema {
    build_schema_with_limits(pool, broadcast_tx, GraphqlLimits::from_env())
}

/// Build the schema with explicit query limits.
pub fn build_schema_with_limits(
    pool: Arc<SqlitePool>,
    broadcast_tx: broadcast::Sender<String>,
    limits: GraphqlLimits,
) -> AppSchema {
    let builder = Schema::build(
        QueryRoot { pool: pool.clone() },
        MutationRoot { pool },
        SubscriptionRoot {
            broadcast_rx: broadcast_tx,
        },
    )
    .limit_depth(limits.max_depth)
    .limit_complexity(limits.max_complexity);

    if limits.introspection {
        builder.finish()
    } else {
        builder.disable_introspection().finish()
    }
}
