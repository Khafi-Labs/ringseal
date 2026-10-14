use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use rand::Rng;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    code_gen::hash_secret,
    errors::AppError,
    models::{Tenant, TenantApiKey},
    AppState,
};

pub struct TenantAuth {
    pub tenant: Tenant,
    pub api_key: TenantApiKey,
}

impl FromRequestParts<Arc<AppState>> for TenantAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
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
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Unauthorized("Invalid API key".into()))?;

        if !tenant.active || !api_key.active {
            return Err(AppError::Unauthorized(
                "API key or tenant is inactive".into(),
            ));
        }

        if let Some(expires_at) = api_key.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(AppError::Unauthorized("API key expired".into()));
            }
        }

        if let Some(allowlist) = &api_key.ip_allowlist {
            let client_ip = extract_client_ip(headers);
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

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let app_state = state;
        let headers = &parts.headers;

        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized(
                "Invalid Authorization header format".into(),
            ));
        }

        let token = &auth_header[7..];
        let master_secret = &app_state.config.master_secret;

        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_required_spec_claims(&["exp"]);
        // The library's own audience check rejects every token that carries an
        // `aud` claim when no expected audience is configured. The audience is
        // checked explicitly below instead: optional, but it must be "ringseal".
        validation.validate_aud = false;

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
        normalized_claims.customer_ref =
            crate::models::normalize_ref(&normalized_claims.customer_ref);

        Ok(CustomerAuth(normalized_claims))
    }
}

/// Header set by the request-guard middleware once it has resolved the real
/// client address. Any value sent by a client is discarded.
pub const CLIENT_IP_HEADER: &str = "x-ringseal-client-ip";

/// Resolve the client address.
///
/// With no trusted proxy the socket peer is used and `X-Forwarded-For` is
/// ignored, because the client controls it. With `n` trusted proxies, each of
/// which appends the address it saw, the client is the n-th entry from the
/// right; anything further left may have been forged by the client.
pub fn resolve_client_ip(peer: IpAddr, xff: Option<&str>, trusted_proxy_count: usize) -> String {
    if trusted_proxy_count == 0 {
        return peer.to_string();
    }
    if let Some(xff) = xff {
        let parts: Vec<&str> = xff
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= trusted_proxy_count {
            let candidate = parts[parts.len() - trusted_proxy_count];
            if candidate.parse::<IpAddr>().is_ok() {
                return candidate.to_string();
            }
        }
    }
    peer.to_string()
}

/// The client address resolved by the request-guard middleware.
pub fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get(CLIENT_IP_HEADER)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
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
    rand::rng().fill_bytes(&mut bytes);
    format!("rsk_{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> IpAddr {
        "203.0.113.9".parse().unwrap()
    }

    #[test]
    fn without_trusted_proxies_the_header_is_ignored() {
        assert_eq!(
            resolve_client_ip(peer(), Some("198.51.100.1"), 0),
            "203.0.113.9"
        );
    }

    #[test]
    fn one_proxy_uses_the_last_entry() {
        assert_eq!(
            resolve_client_ip(peer(), Some("6.6.6.6, 198.51.100.1"), 1),
            "198.51.100.1"
        );
    }

    #[test]
    fn two_proxies_use_the_second_entry_from_the_right() {
        assert_eq!(
            resolve_client_ip(peer(), Some("6.6.6.6, 198.51.100.1, 192.0.2.7"), 2),
            "198.51.100.1"
        );
    }

    #[test]
    fn forged_leading_entries_are_not_trusted() {
        let ip = resolve_client_ip(peer(), Some("1.1.1.1, 2.2.2.2, 198.51.100.1"), 1);
        assert_eq!(ip, "198.51.100.1");
    }

    #[test]
    fn too_few_entries_or_garbage_fall_back_to_the_peer() {
        assert_eq!(
            resolve_client_ip(peer(), Some("198.51.100.1"), 2),
            "203.0.113.9"
        );
        assert_eq!(
            resolve_client_ip(peer(), Some("not-an-ip"), 1),
            "203.0.113.9"
        );
        assert_eq!(resolve_client_ip(peer(), None, 1), "203.0.113.9");
    }

    #[test]
    fn extract_reads_the_internal_header_only() {
        let mut h = HeaderMap::new();
        assert_eq!(extract_client_ip(&h), "unknown");
        h.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        assert_eq!(extract_client_ip(&h), "unknown");
        h.insert(CLIENT_IP_HEADER, "198.51.100.1".parse().unwrap());
        assert_eq!(extract_client_ip(&h), "198.51.100.1");
    }
}
