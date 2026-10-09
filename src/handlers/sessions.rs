use axum::{
    extract::{Path, State, Query},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::{
    audit::log_action,
    auth::{extract_client_ip, extract_user_agent, has_scope, TenantAuth},
    code_gen::{encrypt_code, generate_code},
    errors::AppError,
    AppState,
    models::{
        ActiveSessionQuery, CreateSessionRequest, CreateSessionResponse, SessionResponse, SessionStatus, normalize_ref
    },
};

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: TenantAuth,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, AppError> {
    if !has_scope(&auth.api_key, "create") {
        return Err(AppError::Forbidden("Missing 'create' scope".into()));
    }

    let customer_ref = normalize_ref(&payload.customer_ref);
    let agent_ref = normalize_ref(&payload.agent_ref);

    let recent_count = state.db.count_recent_sessions_for_customer(
        auth.tenant.id, 
        &customer_ref, 
        state.config.max_sessions_window_seconds
    ).await.map_err(AppError::Database)?;

    if recent_count >= state.config.max_sessions_per_customer as i64 {
        return Err(AppError::BadRequest("Max sessions per customer exceeded".into()));
    }

    let code = generate_code();
    let encrypted = encrypt_code(&code, state.config.master_secret.as_bytes(), &auth.tenant.id);
    let expires_at = Utc::now() + chrono::Duration::seconds(state.config.code_expiry_seconds);

    let session = state.db.create_session(
        auth.tenant.id,
        &customer_ref,
        &agent_ref,
        encrypted,
        expires_at,
        state.config.max_attempts,
    ).await.map_err(AppError::Database)?;

    let ip = extract_client_ip(&headers, state.config.trusted_proxy_count);
    let ua = extract_user_agent(&headers);
    let principal = auth.api_key.label.clone();

    log_action(
        &state.db,
        auth.tenant.id,
        Some(session.id),
        "create_session",
        "api_client",
        Some(&principal),
        &ip,
        ua.as_deref(),
        None,
    ).await?;

    Ok(Json(CreateSessionResponse {
        session_id: session.id,
        code,
        expires_at,
    }))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: TenantAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionResponse>, AppError> {
    if !has_scope(&auth.api_key, "read") {
        return Err(AppError::Forbidden("Missing 'read' scope".into()));
    }

    let session = state.db.get_session(id).await.map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    if session.tenant_id != auth.tenant.id {
        return Err(AppError::NotFound("Session not found".into()));
    }

    let ip = extract_client_ip(&headers, state.config.trusted_proxy_count);
    let ua = extract_user_agent(&headers);
    let principal = auth.api_key.label.clone();

    log_action(
        &state.db,
        auth.tenant.id,
        Some(session.id),
        "get_session",
        "api_client",
        Some(&principal),
        &ip,
        ua.as_deref(),
        None,
    ).await?;

    Ok(Json(SessionResponse {
        id: session.id,
        tenant_id: session.tenant_id,
        customer_ref: session.customer_ref,
        agent_ref: session.agent_ref,
        status: session.status.parse().unwrap_or(SessionStatus::Ended),
        created_at: session.created_at,
        ended_at: session.ended_at,
    }))
}

pub async fn get_active(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: TenantAuth,
    Query(query): Query<ActiveSessionQuery>,
) -> Result<Json<SessionResponse>, AppError> {
    if !has_scope(&auth.api_key, "read") {
        return Err(AppError::Forbidden("Missing 'read' scope".into()));
    }

    let customer_ref = normalize_ref(&query.customer_ref);

    let session = state.db.get_active_session_for_customer(auth.tenant.id, &customer_ref)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Active session not found".into()))?;

    let ip = extract_client_ip(&headers, state.config.trusted_proxy_count);
    let ua = extract_user_agent(&headers);
    let principal = auth.api_key.label.clone();

    log_action(
        &state.db,
        auth.tenant.id,
        Some(session.id),
        "get_active_session",
        "api_client",
        Some(&principal),
        &ip,
        ua.as_deref(),
        None,
    ).await?;

    Ok(Json(SessionResponse {
        id: session.id,
        tenant_id: session.tenant_id,
        customer_ref: session.customer_ref,
        agent_ref: session.agent_ref,
        status: session.status.parse().unwrap_or(SessionStatus::Ended),
        created_at: session.created_at,
        ended_at: session.ended_at,
    }))
}

pub async fn end(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: TenantAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, AppError> {
    if !has_scope(&auth.api_key, "end") {
        return Err(AppError::Forbidden("Missing 'end' scope".into()));
    }

    let session = state.db.get_session(id).await.map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    if session.tenant_id != auth.tenant.id {
        return Err(AppError::NotFound("Session not found".into()));
    }

    state.db.end_session(id).await.map_err(AppError::Database)?;

    let ip = extract_client_ip(&headers, state.config.trusted_proxy_count);
    let ua = extract_user_agent(&headers);
    let principal = auth.api_key.label.clone();

    log_action(
        &state.db,
        auth.tenant.id,
        Some(session.id),
        "end_session",
        "api_client",
        Some(&principal),
        &ip,
        ua.as_deref(),
        None,
    ).await?;

    Ok(Json(()))
}
