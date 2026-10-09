use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::AppState;

pub mod health;
pub mod sessions;
pub mod tenants;
pub mod verification;

pub fn public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/sessions", post(sessions::create))
        .route("/api/v1/sessions/active", get(sessions::get_active))
        .route("/api/v1/sessions/{id}", get(sessions::get))
        .route("/api/v1/sessions/{id}/end", post(sessions::end))
        .route("/api/v1/verify", get(verification::get_active_session))
        .route("/api/v1/health", get(health::check))
}

pub fn internal_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/internal/v1/tenants", post(tenants::create))
        .route("/internal/v1/tenants/{tenant_id}/keys", post(tenants::create_api_key))
        .route("/internal/v1/tenants/{tenant_id}/keys", get(tenants::list_api_keys))
        .route("/internal/v1/tenants/{tenant_id}/keys/{key_id}/revoke", post(tenants::revoke_api_key))
        .route("/internal/v1/health", get(health::check))
}
