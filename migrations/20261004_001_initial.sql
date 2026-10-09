-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_key_hash VARCHAR(255) NOT NULL UNIQUE,
    webhook_url TEXT,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_api_key_hash ON tenants(api_key_hash);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    customer_ref VARCHAR(255) NOT NULL,
    agent_ref VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'created',
    code_hash VARCHAR(255) NOT NULL,
    code_encrypted BYTEA NOT NULL,
    code_expires_at TIMESTAMPTZ NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_tenant_customer ON sessions(tenant_id, customer_ref);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_expires ON sessions(code_expires_at) WHERE status IN ('created', 'active');

-- Audit log table (append-only)
CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    session_id UUID REFERENCES sessions(id),
    action VARCHAR(100) NOT NULL,
    actor VARCHAR(255) NOT NULL,
    ip_address VARCHAR(45) NOT NULL,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_tenant ON audit_log(tenant_id);
CREATE INDEX idx_audit_session ON audit_log(session_id);
CREATE INDEX idx_audit_created ON audit_log(created_at);

-- Prevent UPDATE and DELETE on audit_log (append-only enforcement)
CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only: modifications are not allowed';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_no_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();

CREATE TRIGGER audit_no_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
