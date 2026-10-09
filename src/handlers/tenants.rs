use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use subtle::ConstantTimeEq;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::generate_api_key,
    code_gen::hash_secret,
    errors::AppError,
    AppState,
    models::{CreateApiKeyRequest, CreateApiKeyResponse, CreateTenantRequest, CreateTenantResponse, TenantApiKey},
};

fn check_admin_secret(headers: &HeaderMap, config_secret: &str) -> Result<(), AppError> {
    let header_val = headers
        .get("X-Admin-Secret")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let expected = config_secret.as_bytes();
    let provided = header_val.as_bytes();

    if expected.len() != provided.len() || !bool::from(expected.ct_eq(provided)) {
        return Err(AppError::Unauthorized("Invalid admin secret".into()));
    }

    Ok(())
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<CreateTenantResponse>, AppError> {
    check_admin_secret(&headers, &state.config.admin_secret)?;

    let tenant = state.db.create_tenant(&payload.name, payload.webhook_url.as_deref())
        .await
        .map_err(AppError::Database)?;

    let api_key_str = generate_api_key();
    let key_hash = hash_secret(&api_key_str);

    let _api_key = state.db.create_api_key(
        tenant.id,
        &key_hash,
        "default",
        "create,read,end",
        None,
        None,
    ).await.map_err(AppError::Database)?;

    Ok(Json(CreateTenantResponse {
        id: tenant.id,
        name: tenant.name,
        api_key: api_key_str,
    }))
}

pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    check_admin_secret(&headers, &state.config.admin_secret)?;

    let api_key_str = generate_api_key();
    let key_hash = hash_secret(&api_key_str);

    let scopes_str = payload.scopes.unwrap_or_else(|| vec!["read".to_string()]).join(",");
    let ip_allowlist_str = payload.ip_allowlist.map(|list| list.join(","));

    let api_key = state.db.create_api_key(
        tenant_id,
        &key_hash,
        &payload.label,
        &scopes_str,
        payload.expires_at,
        ip_allowlist_str.as_deref(),
    ).await.map_err(AppError::Database)?;

    Ok(Json(CreateApiKeyResponse {
        id: api_key.id,
        api_key: api_key_str,
        label: api_key.label,
        scopes: scopes_str.split(',').map(|s| s.to_string()).collect(),
        expires_at: api_key.expires_at,
    }))
}

pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<TenantApiKey>>, AppError> {
    check_admin_secret(&headers, &state.config.admin_secret)?;

    let keys = state.db.list_api_keys(tenant_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(keys))
}

pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((tenant_id, key_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, AppError> {
    check_admin_secret(&headers, &state.config.admin_secret)?;

    state.db.revoke_api_key(key_id, tenant_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(()))
}
