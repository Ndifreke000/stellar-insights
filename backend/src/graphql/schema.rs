use async_graphql::{EmptySubscription, Schema};
use sqlx::SqlitePool;
use std::sync::Arc;

use super::resolvers::{MutationRoot, QueryRoot};

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(pool: Arc<SqlitePool>) -> AppSchema {
    Schema::build(
        QueryRoot { pool: pool.clone() },
        MutationRoot { pool },
        EmptySubscription,
    )
    .finish()
}
