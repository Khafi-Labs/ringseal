use crate::{
    db::Database,
    errors::AppError,
    models::AuditEntry,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub async fn log_action(
    db: &Database,
    tenant_id: Uuid,
    session_id: Option<Uuid>,
    action: &str,
    actor: &str,
    authenticated_principal: Option<&str>,
    ip: &str,
    user_agent: Option<&str>,
    details: Option<serde_json::Value>,
) -> Result<(), AppError> {
    let latest_hash = db.get_latest_audit_hash(tenant_id).await.map_err(AppError::Database)?;

    let now = chrono::Utc::now();
    let entry_id = Uuid::new_v4();

    let entry = AuditEntry {
        id: entry_id,
        tenant_id,
        session_id,
        action: action.to_string(),
        actor: actor.to_string(),
        authenticated_principal: authenticated_principal.map(|s| s.to_string()),
        ip_address: ip.to_string(),
        user_agent: user_agent.map(|s| s.to_string()),
        details,
        prev_hash: latest_hash.clone(),
        created_at: now,
    };

    db.insert_audit(&entry).await.map_err(AppError::Database)?;

    Ok(())
}

pub fn compute_entry_hash(entry: &AuditEntry) -> String {
    let mut hasher = Sha256::new();
    let prev = entry.prev_hash.as_deref().unwrap_or("");
    let data = format!(
        "{}|{}|{}|{}|{}|{}",
        entry.id,
        entry.tenant_id,
        entry.action,
        entry.actor,
        entry.created_at.to_rfc3339(),
        prev
    );
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
