use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::models::{AuditEntry, Session, SessionStatus, Tenant, TenantApiKey};

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenants (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                webhook_url TEXT,
                active BOOL NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS tenant_api_keys (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id),
                key_hash TEXT NOT NULL UNIQUE,
                label TEXT NOT NULL,
                scopes TEXT NOT NULL DEFAULT 'create,read,end',
                expires_at TIMESTAMPTZ,
                last_used_at TIMESTAMPTZ,
                ip_allowlist TEXT,
                active BOOL NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS sessions (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id),
                customer_ref TEXT NOT NULL,
                agent_ref TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                code_encrypted BYTEA,
                code_expires_at TIMESTAMPTZ NOT NULL,
                attempts INT NOT NULL DEFAULT 0,
                max_attempts INT NOT NULL DEFAULT 3,
                created_at TIMESTAMPTZ NOT NULL,
                ended_at TIMESTAMPTZ
            );
            
            CREATE TABLE IF NOT EXISTS audit_log (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL,
                session_id UUID,
                action TEXT NOT NULL,
                actor TEXT NOT NULL,
                authenticated_principal TEXT,
                ip_address TEXT NOT NULL,
                user_agent TEXT,
                details JSONB,
                prev_hash TEXT,
                created_at TIMESTAMPTZ NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tenant_api_keys_hash ON tenant_api_keys(key_hash);
            CREATE INDEX IF NOT EXISTS idx_sessions_tenant_customer ON sessions(tenant_id, customer_ref);
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(code_expires_at) WHERE status IN ('created', 'active');
            CREATE INDEX IF NOT EXISTS idx_audit_log_tenant ON audit_log(tenant_id);
            CREATE INDEX IF NOT EXISTS idx_audit_log_session ON audit_log(session_id);
            CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn create_tenant(&self, name: &str, webhook_url: Option<&str>) -> Result<Tenant, sqlx::Error> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        
        sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (id, name, webhook_url, active, created_at)
            VALUES ($1, $2, $3, true, $4)
            RETURNING id, name, webhook_url, active, created_at
            "#
        )
        .bind(id)
        .bind(name)
        .bind(webhook_url)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_tenant_by_id(&self, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_api_key(
        &self,
        tenant_id: Uuid,
        key_hash: &str,
        label: &str,
        scopes: &str,
        expires_at: Option<DateTime<Utc>>,
        ip_allowlist: Option<&str>,
    ) -> Result<TenantApiKey, sqlx::Error> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        
        sqlx::query_as::<_, TenantApiKey>(
            r#"
            INSERT INTO tenant_api_keys (id, tenant_id, key_hash, label, scopes, expires_at, ip_allowlist, active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8)
            RETURNING id, tenant_id, key_hash, label, scopes, expires_at, last_used_at, ip_allowlist, active, created_at
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .bind(key_hash)
        .bind(label)
        .bind(scopes)
        .bind(expires_at)
        .bind(ip_allowlist)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<(Tenant, TenantApiKey)>, sqlx::Error> {
        let key = sqlx::query_as::<_, TenantApiKey>("SELECT * FROM tenant_api_keys WHERE key_hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(k) = key {
            let tenant = self.get_tenant_by_id(k.tenant_id).await?;
            if let Some(t) = tenant {
                return Ok(Some((t, k)));
            }
        }
        
        Ok(None)
    }

    pub async fn revoke_api_key(&self, key_id: Uuid, tenant_id: Uuid) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("UPDATE tenant_api_keys SET active = false WHERE id = $1 AND tenant_id = $2 AND active = true")
            .bind(key_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
            
        Ok(res.rows_affected() > 0)
    }

    pub async fn update_api_key_last_used(&self, key_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tenant_api_keys SET last_used_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(key_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_api_keys(&self, tenant_id: Uuid) -> Result<Vec<TenantApiKey>, sqlx::Error> {
        sqlx::query_as::<_, TenantApiKey>("SELECT * FROM tenant_api_keys WHERE tenant_id = $1 ORDER BY created_at DESC")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn create_session(
        &self,
        tenant_id: Uuid,
        customer_ref: &str,
        agent_ref: &str,
        code_encrypted: Vec<u8>,
        expires_at: DateTime<Utc>,
        max_attempts: i32,
    ) -> Result<Session, sqlx::Error> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        
        sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (id, tenant_id, customer_ref, agent_ref, status, code_encrypted, code_expires_at, attempts, max_attempts, created_at)
            VALUES ($1, $2, $3, $4, 'created', $5, $6, 0, $7, $8)
            RETURNING id, tenant_id, customer_ref, agent_ref, status, code_encrypted, code_expires_at, attempts, max_attempts, created_at, ended_at
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .bind(customer_ref)
        .bind(agent_ref)
        .bind(code_encrypted)
        .bind(expires_at)
        .bind(max_attempts)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_active_session_for_customer(&self, tenant_id: Uuid, customer_ref: &str) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE tenant_id = $1 AND customer_ref = $2 AND status IN ('created', 'active') LIMIT 1"
        )
        .bind(tenant_id)
        .bind(customer_ref)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_session_status(&self, id: Uuid, status: SessionStatus) -> Result<(), sqlx::Error> {
        let status_str = status.to_string();
        sqlx::query("UPDATE sessions SET status = $1 WHERE id = $2")
            .bind(status_str)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn increment_session_attempts(&self, id: Uuid) -> Result<i32, sqlx::Error> {
        let (attempts,): (i32,) = sqlx::query_as(
            "UPDATE sessions SET attempts = attempts + 1 WHERE id = $1 RETURNING attempts"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(attempts)
    }

    pub async fn end_session(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE sessions SET status = 'ended', ended_at = NOW(), code_encrypted = NULL WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn expire_stale_sessions(&self) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(
            "UPDATE sessions SET status = 'expired', ended_at = NOW(), code_encrypted = NULL WHERE status IN ('created', 'active') AND code_expires_at < NOW()"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(res.rows_affected())
    }

    pub async fn count_recent_sessions_for_customer(&self, tenant_id: Uuid, customer_ref: &str, window_seconds: i64) -> Result<i64, sqlx::Error> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sessions WHERE tenant_id = $1 AND customer_ref = $2 AND created_at > NOW() - interval '1 second' * $3"
        )
        .bind(tenant_id)
        .bind(customer_ref)
        .bind(window_seconds as f64)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count)
    }

    pub async fn insert_audit(&self, entry: &AuditEntry) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audit_log (id, tenant_id, session_id, action, actor, authenticated_principal, ip_address, user_agent, details, prev_hash, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#
        )
        .bind(entry.id)
        .bind(entry.tenant_id)
        .bind(entry.session_id)
        .bind(&entry.action)
        .bind(&entry.actor)
        .bind(&entry.authenticated_principal)
        .bind(&entry.ip_address)
        .bind(&entry.user_agent)
        .bind(&entry.details)
        .bind(&entry.prev_hash)
        .bind(entry.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_latest_audit_hash(&self, tenant_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct LatestAudit {
            id: Uuid,
            tenant_id: Uuid,
            action: String,
            actor: String,
            created_at: DateTime<Utc>,
            prev_hash: Option<String>,
        }
        
        let latest = sqlx::query_as::<_, LatestAudit>(
            "SELECT id, tenant_id, action, actor, created_at, prev_hash FROM audit_log WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(l) = latest {
            let mut hasher = Sha256::new();
            let data = format!(
                "{}:{}:{}:{}:{}:{}",
                l.id, l.tenant_id, l.action, l.actor, l.created_at.timestamp(), l.prev_hash.unwrap_or_default()
            );
            hasher.update(data.as_bytes());
            Ok(Some(hex::encode(hasher.finalize())))
        } else {
            Ok(None)
        }
    }

    pub async fn get_audit_log(&self, tenant_id: Uuid, session_id: Option<Uuid>, limit: i64) -> Result<Vec<AuditEntry>, sqlx::Error> {
        if let Some(sid) = session_id {
            sqlx::query_as::<_, AuditEntry>("SELECT * FROM audit_log WHERE tenant_id = $1 AND session_id = $2 ORDER BY created_at DESC LIMIT $3")
                .bind(tenant_id)
                .bind(sid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, AuditEntry>("SELECT * FROM audit_log WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2")
                .bind(tenant_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        }
    }
}
