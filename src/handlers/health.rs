use axum::Json;
use chrono::Utc;
use crate::models::HealthResponse;

pub async fn check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(), // Assuming standard cargo pkg version available
        timestamp: Utc::now(),
    })
}
