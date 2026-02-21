mod middleware;
mod api;
mod validators;
mod errors;

use actix_web::{web, App, HttpServer, middleware::Logger};
use env_logger::Env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Starting API server with validation middleware");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(middleware::rate_limit::RateLimitMiddleware::new(100, std::time::Duration::from_secs(60)))
            // Register API routes
            .service(
                web::scope("/api/v1")
                    .route("/corridors", web::get().to(api::handlers::list_corridors))
                    .route("/corridors/{id}", web::get().to(api::handlers::get_corridor))
                    .route("/corridors", web::post().to(api::handlers::create_corridor))
                    .route("/corridors/{id}", web::put().to(api::handlers::update_corridor))
                    .route("/corridors/{id}", web::delete().to(api::handlers::delete_corridor))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
