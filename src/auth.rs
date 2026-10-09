use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use rand::RngCore;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    code_gen::hash_secret,
    errors::AppError,
    AppState,
    models::{Tenant, TenantApiKey},
};

pub struct TenantAuth {
    pub tenant: Tenant,
    pub api_key: TenantApiKey,
}

impl FromRequestParts<Arc<AppState>> for TenantAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let app_state = state;
        let headers = &parts.headers;

        let api_key_str = headers
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing X-API-Key header".into()))?;

        let hashed_key = hash_secret(api_key_str);

        let (tenant, api_key) = app_state
            .db
            .get_api_key_by_hash(&hashed_key)
            .await
            .map_err(|e| AppError::Database(e))?
            .ok_or_else(|| AppError::Unauthorized("Invalid API key".into()))?;

        if !tenant.active || !api_key.active {
            return Err(AppError::Unauthorized("API key or tenant is inactive".into()));
        }

        if let Some(expires_at) = api_key.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(AppError::Unauthorized("API key expired".into()));
            }
        }

        if let Some(allowlist) = &api_key.ip_allowlist {
            let client_ip = extract_client_ip(headers, app_state.config.trusted_proxy_count);
            if !allowlist.split(',').any(|ip| ip.trim() == client_ip) {
                return Err(AppError::Forbidden("IP address not in allowlist".into()));
            }
        }

        let db = app_state.db.clone();
        let key_id = api_key.id;
        tokio::spawn(async move {
            let _ = db.update_api_key_last_used(key_id).await;
        });

        Ok(TenantAuth { tenant, api_key })
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub tenant_id: Uuid,
    pub customer_ref: String,
    pub exp: usize,
    pub iat: Option<usize>,
    pub aud: Option<String>,
    pub jti: Option<String>,
}

pub struct CustomerAuth(pub Claims);

impl FromRequestParts<Arc<AppState>> for CustomerAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let app_state = state;
        let headers = &parts.headers;

        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized("Invalid Authorization header format".into()));
        }

        let token = &auth_header[7..];
        let master_secret = &app_state.config.master_secret;

        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_required_spec_claims(&["exp"]);

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(master_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

        let claims = token_data.claims;

        if let Some(iat) = claims.iat {
            let current_time = chrono::Utc::now().timestamp() as usize;
            let lifetime = (current_time.saturating_sub(iat)) as i64;
            if lifetime > app_state.config.max_jwt_lifetime_seconds {
                return Err(AppError::Unauthorized("Token lifetime exceeded".into()));
            }
        }

        if let Some(aud) = &claims.aud {
            if aud != "ringseal" {
                return Err(AppError::Unauthorized("Invalid token audience".into()));
            }
        }

        let mut normalized_claims = claims;
        normalized_claims.customer_ref = crate::models::normalize_ref(&normalized_claims.customer_ref);

        Ok(CustomerAuth(normalized_claims))
    }
}

pub fn extract_client_ip(headers: &HeaderMap, trusted_proxy_count: usize) -> String {
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|h| h.to_str().ok()) {
        let parts: Vec<&str> = xff.split(',').map(|s| s.trim()).collect();
        if parts.len() > trusted_proxy_count {
            return parts[parts.len() - 1 - trusted_proxy_count].to_string();
        }
    }
    "unknown".to_string()
}

pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

pub fn has_scope(key: &TenantApiKey, scope: &str) -> bool {
    key.scopes.split(',').map(|s| s.trim()).any(|s| s == scope)
}

pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("rsk_{}", hex::encode(bytes))
}
