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

/// Build the consolidated GraphQL schema with database access.
///
/// This is the primary schema builder used in production. It includes:
/// - Query resolvers for all entities (anchors, corridors, payments, etc.)
/// - Mutation resolvers for create/update/delete operations
/// - Subscription resolvers for real-time updates via WebSocket
pub fn build_schema(pool: Arc<SqlitePool>, broadcast_tx: broadcast::Sender<String>) -> AppSchema {
    Schema::build(
        QueryRoot { pool: pool.clone() },
        MutationRoot { pool },
        SubscriptionRoot {
            broadcast_rx: broadcast_tx,
        },
    )
    .finish()
}
