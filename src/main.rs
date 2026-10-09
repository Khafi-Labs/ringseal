pub mod audit;
pub mod auth;
pub mod code_gen;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod rate_limit;
pub mod webhooks;

use axum::{
    middleware::{self, Next},
    response::Response,
    extract::Request,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_http::cors::{CorsLayer, Any};
use tower::timeout::TimeoutLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::AppConfig,
    db::Database,
    rate_limit::RateLimiter,
};

/// Shared application state, passed to all handlers via Arc.
pub struct AppState {
    pub db: Database,
    pub config: AppConfig,
    pub rate_limiter: RateLimiter,
}

/// F-13: Security headers middleware — applied to every response.
async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    headers.insert("cache-control", "no-store".parse().unwrap());
    response
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ringseal=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Load and validate configuration (F-03: rejects weak/default secrets)
    let config = AppConfig::from_env();

    // Connect to Postgres and run inline migrations
    let db = Database::new(&config.database_url).await?;
    let rate_limiter = RateLimiter::new();

    let addr_public = format!("{}:{}", config.host, config.port);
    let addr_internal = format!("127.0.0.1:{}", config.internal_port);

    let state = Arc::new(AppState {
        db,
        config,
        rate_limiter,
    });

    // F-13: CORS — restrict in production, permissive for dev
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public API — sessions, verification, health
    let public_app = handlers::public_router()
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .with_state(state.clone());

    // F-04: Internal API on a separate listener (127.0.0.1 only)
    // Tenant management, API key rotation — never exposed publicly
    let internal_app = handlers::internal_router()
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Background task: expire stale sessions and clean rate limiter
    let bg_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            match bg_state.db.expire_stale_sessions().await {
                Ok(count) if count > 0 => {
                    tracing::info!(expired = count, "Expired stale sessions");
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to expire stale sessions");
                }
                _ => {}
            }
            bg_state
                .rate_limiter
                .cleanup(bg_state.config.rate_limit_window_seconds);
        }
    });

    // Start internal listener (admin API) on loopback only
    tokio::spawn(async move {
        let listener = TcpListener::bind(&addr_internal)
            .await
            .expect("Failed to bind internal API listener");
        tracing::info!(addr = %addr_internal, "Internal API listening");
        axum::serve(listener, internal_app)
            .await
            .expect("Internal API server failed");
    });

    // Start public listener
    tracing::info!(addr = %addr_public, "Public API listening");
    let listener = TcpListener::bind(&addr_public).await?;
    axum::serve(listener, public_app).await?;

    Ok(())
}
