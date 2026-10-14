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
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::AppConfig, db::Database, rate_limit::RateLimiter};

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

/// Resolves the real client address, then enforces rate limits.
///
/// Limits are per client IP and per presented credential (hashed, so no secret
/// is kept in memory). Requests with no credential also count against the
/// stricter anonymous limit. The health check is exempt.
async fn request_guard_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let xff = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok());
    let ip = auth::resolve_client_ip(peer.ip(), xff, state.config.trusted_proxy_count);

    // Never trust a client-supplied value for the internal header.
    req.headers_mut().remove(auth::CLIENT_IP_HEADER);
    if let Ok(value) = HeaderValue::from_str(&ip) {
        req.headers_mut()
            .insert(HeaderName::from_static(auth::CLIENT_IP_HEADER), value);
    }

    if req.uri().path() != "/api/v1/health" {
        if let Err(err) = check_rate_limits(&state, &ip, req.headers()) {
            let mut response = err.into_response();
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from(state.config.rate_limit_window_seconds),
            );
            return response;
        }
    }

    next.run(req).await
}

fn check_rate_limits(
    state: &AppState,
    ip: &str,
    headers: &HeaderMap,
) -> Result<(), errors::AppError> {
    let cfg = &state.config;
    state.rate_limiter.check_ip_rate_limit(
        ip,
        cfg.rate_limit_requests,
        cfg.rate_limit_window_seconds,
    )?;

    let credential = headers
        .get("x-api-key")
        .or_else(|| headers.get(header::AUTHORIZATION))
        .and_then(|h| h.to_str().ok());

    match credential {
        Some(credential) => {
            let digest = code_gen::hash_secret(credential);
            state.rate_limiter.check_rate_limit(
                &format!("cred:{}", &digest[..16]),
                cfg.rate_limit_requests,
                cfg.rate_limit_window_seconds,
            )
        }
        None => state.rate_limiter.check_rate_limit(
            &format!("anon:{ip}"),
            cfg.unauth_rate_limit_requests,
            cfg.unauth_rate_limit_window_seconds,
        ),
    }
}

/// CORS for browser callers (the demo pages). Closed unless origins are
/// configured with `CORS_ALLOWED_ORIGINS`; `*` allows any origin.
fn build_cors(origins: &[String]) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-api-key"),
        ]);
    if origins.iter().any(|o| o == "*") {
        base.allow_origin(Any)
    } else {
        let allowed: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
        base.allow_origin(allowed)
    }
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

    // F-13: CORS is closed unless CORS_ALLOWED_ORIGINS lists origins.
    let cors = build_cors(&state.config.cors_allowed_origins);

    // Public API — sessions, verification, health
    let public_app = handlers::public_router()
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        // Outermost: resolves the client address and enforces rate limits
        // before any other layer or handler runs.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_guard_middleware,
        ))
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
    axum::serve(
        listener,
        public_app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
