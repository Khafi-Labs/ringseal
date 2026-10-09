use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use std::sync::Arc;
use chrono::Utc;

use crate::{
    audit::log_action,
    auth::{extract_client_ip, extract_user_agent, CustomerAuth},
    code_gen::decrypt_code,
    errors::AppError,
    AppState,
    models::{SessionStatus, VerifySessionResponse, WebhookPayload},
    webhooks::spawn_webhook,
};

pub async fn get_active_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: CustomerAuth,
) -> Result<Json<VerifySessionResponse>, AppError> {
    let claims = auth.0;

    let session = state.db.get_active_session_for_customer(claims.tenant_id, &claims.customer_ref)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("No active session".into()))?;

    let tenant = state.db.get_tenant_by_id(claims.tenant_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Tenant not found".into()))?;

    let ip = extract_client_ip(&headers, state.config.trusted_proxy_count);
    let ua = extract_user_agent(&headers);

    log_action(
        &state.db,
        claims.tenant_id,
        Some(session.id),
        "view_session",
        "customer",
        Some(&claims.customer_ref),
        &ip,
        ua.as_deref(),
        Some(serde_json::json!({
            "device_info": ua.clone().unwrap_or_default()
        })),
    ).await?;

    if let Some(webhook_url) = tenant.webhook_url {
        let payload = WebhookPayload {
            event_id: uuid::Uuid::new_v4(),
            event: "session.viewed".into(),
            tenant_id: claims.tenant_id,
            session_id: session.id,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "ip_address": ip,
                "user_agent": ua,
            }),
        };
        spawn_webhook(webhook_url, payload, state.config.master_secret.clone());
    }

    state.db.increment_session_attempts(session.id).await.map_err(AppError::Database)?;

    if session.attempts >= session.max_attempts {
        let _ = state.db.update_session_status(session.id, SessionStatus::Expired).await;
        return Err(AppError::MaxAttemptsExceeded);
    }

    if Utc::now() > session.code_expires_at {
        let _ = state.db.update_session_status(session.id, SessionStatus::Expired).await;
        return Err(AppError::SessionExpired);
    }

    let encrypted = session.code_encrypted.ok_or_else(|| AppError::Internal("Missing encrypted code".into()))?;
    let code = decrypt_code(&encrypted, state.config.master_secret.as_bytes(), &claims.tenant_id)?;

    Ok(Json(VerifySessionResponse {
        session_id: session.id,
        code,
        expires_at: session.code_expires_at,
        status: session.status.parse().unwrap_or(SessionStatus::Active),
    }))
}
