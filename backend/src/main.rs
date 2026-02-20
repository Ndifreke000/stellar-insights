// Build transaction routes
let transaction_routes = Router::new()
    .nest(
        "/api/transactions",
        stellar_insights_backend::api::transactions::routes(),
    )
    .with_state(app_state.clone())
    .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
        rate_limiter.clone(),
        rate_limit_middleware,
    )))
    .layer(cors.clone());

// Build trustline routes
let trustline_routes = Router::new()
    .nest(
        "/api/trustlines",
        stellar_insights_backend::api::trustlines::routes(Arc::clone(&trustline_analyzer)),
    )
    .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
        rate_limiter.clone(),
        rate_limit_middleware,
    )))
    .layer(cors.clone());