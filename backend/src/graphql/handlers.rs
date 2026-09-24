use async_graphql::http::GraphiQLSource;
use async_graphql::Schema;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use super::schema::AppSchema;
use super::resolvers::{MutationRoot, QueryRoot};
use super::subscription::SubscriptionRoot;

/// GraphQL query/mutation handler.
///
/// Accepts POST requests with JSON body containing `query`, `variables`, and `operationName`.
pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    Json(schema.execute(req.0).await.into())
}

/// GraphQL Playground (GraphiQL) handler.
///
/// Serves the GraphiQL interactive IDE for exploring the GraphQL schema.
pub async fn graphql_playground() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

/// GraphQL WebSocket handler for subscriptions.
///
/// Upgrades HTTP connections to WebSocket for real-time subscription streams.
pub async fn graphql_ws_handler(
    State(schema): State<AppSchema>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let mut service = async_graphql_axum::GraphQLSubscription::new(schema);
    match tower::ServiceExt::oneshot(&mut service, req).await {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Health check endpoint for the GraphQL API.
///
/// Returns the status of the GraphQL API and its dependencies.
pub async fn graphql_health_handler(
    State(schema): State<AppSchema>,
) -> Json<GraphQLHealthResponse> {
    // Execute a simple introspection query to verify the schema is functional
    let result = schema.execute("{ __typename }").await;

    Json(GraphQLHealthResponse {
        status: if result.errors.is_empty() {
            "healthy"
        } else {
            "degraded"
        },
        version: env!("CARGO_PKG_VERSION"),
        endpoint: "/graphql",
        ws_endpoint: "/graphql/ws",
    })
}

#[derive(Serialize)]
pub struct GraphQLHealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub endpoint: &'static str,
    pub ws_endpoint: &'static str,
}
