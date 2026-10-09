use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use std::fmt;
use std::str::FromStr;
use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Active,
    Verified,
    Expired,
    Ended,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SessionStatus::Created => "created",
            SessionStatus::Active => "active",
            SessionStatus::Verified => "verified",
            SessionStatus::Expired => "expired",
            SessionStatus::Ended => "ended",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for SessionStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(SessionStatus::Created),
            "active" => Ok(SessionStatus::Active),
            "verified" => Ok(SessionStatus::Verified),
            "expired" => Ok(SessionStatus::Expired),
            "ended" => Ok(SessionStatus::Ended),
            _ => Err(AppError::Internal("Invalid session status".to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub webhook_url: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct TenantApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub key_hash: String,
    pub label: String,
    pub scopes: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub ip_allowlist: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_ref: String,
    pub agent_ref: String,
    pub status: String,
    pub code_encrypted: Option<Vec<u8>>,
    pub code_expires_at: DateTime<Utc>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn to_response(&self) -> Result<SessionResponse, AppError> {
        let status = SessionStatus::from_str(&self.status)?;
        Ok(SessionResponse {
            id: self.id,
            tenant_id: self.tenant_id,
            customer_ref: self.customer_ref.clone(),
            agent_ref: self.agent_ref.clone(),
            status,
            created_at: self.created_at,
            ended_at: self.ended_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AuditEntry {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub session_id: Option<Uuid>,
    pub action: String,
    pub actor: String,
    pub authenticated_principal: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub details: Option<serde_json::Value>,
    pub prev_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub customer_ref: String,
    pub agent_ref: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_ref: String,
    pub agent_ref: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct VerifySessionResponse {
    pub session_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub status: SessionStatus,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTenantResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub label: String,
    pub scopes: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub ip_allowlist: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub api_key: String,
    pub label: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeApiKeyRequest {
    pub key_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub event_id: Uuid,
    pub event: String,
    pub tenant_id: Uuid,
    pub session_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ActiveSessionQuery {
    pub customer_ref: String,
}

pub fn normalize_ref(s: &str) -> String {
    s.trim().to_lowercase()
}
