-- F-01: Remove code_hash column from sessions
ALTER TABLE sessions DROP COLUMN IF EXISTS code_hash;

-- F-01/F-11: Make code_encrypted nullable (wiped on end/expiry)
ALTER TABLE sessions ALTER COLUMN code_encrypted DROP NOT NULL;

-- F-02: Create tenant_api_keys table
CREATE TABLE IF NOT EXISTS tenant_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    label VARCHAR(255) NOT NULL DEFAULT 'default',
    scopes TEXT NOT NULL DEFAULT 'create,read,end',
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    ip_allowlist TEXT,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON tenant_api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_tenant ON tenant_api_keys(tenant_id);

-- F-02: Migrate existing api_key_hash from tenants to tenant_api_keys
-- (Only if api_key_hash column exists on tenants)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'tenants' AND column_name = 'api_key_hash'
    ) THEN
        INSERT INTO tenant_api_keys (id, tenant_id, key_hash, label, scopes, created_at)
        SELECT gen_random_uuid(), id, api_key_hash, 'legacy-default', 'create,read,end', created_at
        FROM tenants
        WHERE api_key_hash IS NOT NULL
        ON CONFLICT (key_hash) DO NOTHING;
        
        ALTER TABLE tenants DROP COLUMN api_key_hash;
    END IF;
END $$;

-- F-06: Add hash chain and authenticated_principal to audit_log
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS prev_hash VARCHAR(255);
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS authenticated_principal VARCHAR(255);
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS user_agent TEXT;

-- F-06: Create restricted app role for audit log
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ringseal_app') THEN
        CREATE ROLE ringseal_app WITH LOGIN;
    END IF;
END $$;

-- Grant appropriate permissions
GRANT SELECT, INSERT ON audit_log TO ringseal_app;
GRANT SELECT, INSERT, UPDATE ON tenants TO ringseal_app;
GRANT SELECT, INSERT, UPDATE ON tenant_api_keys TO ringseal_app;
GRANT SELECT, INSERT, UPDATE ON sessions TO ringseal_app;
-- Note: the ringseal_app role CANNOT update/delete audit_log

-- Recreate append-only triggers if they were on audit_log
DROP TRIGGER IF EXISTS audit_no_update ON audit_log;
DROP TRIGGER IF EXISTS audit_no_delete ON audit_log;

CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only: modifications are not allowed';
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_no_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();

CREATE TRIGGER audit_no_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();

-- Ensure only one active session per customer per tenant
CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_active_customer 
    ON sessions(tenant_id, customer_ref) 
    WHERE status IN ('created', 'active');
