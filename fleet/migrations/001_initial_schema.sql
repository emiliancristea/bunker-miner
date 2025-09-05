-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create users table
CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Create index on email for fast lookups
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_active ON users(is_active) WHERE is_active = TRUE;

-- Create rig status enum
CREATE TYPE rig_status AS ENUM ('online', 'offline', 'mining', 'idle', 'error', 'maintenance');

-- Create rigs table
CREATE TABLE rigs (
    rig_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    rig_name VARCHAR(255) NOT NULL,
    description TEXT,
    location VARCHAR(255),
    status rig_status NOT NULL DEFAULT 'offline',
    last_seen TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Create indexes for rigs table
CREATE INDEX idx_rigs_owner ON rigs(owner_user_id);
CREATE INDEX idx_rigs_status ON rigs(status);
CREATE INDEX idx_rigs_active ON rigs(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_rigs_last_seen ON rigs(last_seen);

-- Create API keys table for rig authentication
CREATE TABLE api_keys (
    key_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    key_name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(255) NOT NULL, -- Argon2 hash of the actual API key
    key_prefix VARCHAR(8) NOT NULL, -- First 8 chars for display (bk_abc123)
    rig_id UUID REFERENCES rigs(rig_id) ON DELETE SET NULL, -- Optional association
    permissions TEXT[] NOT NULL DEFAULT '{}', -- Array of permission strings
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_used TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for API keys table
CREATE INDEX idx_api_keys_owner ON api_keys(owner_user_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_active ON api_keys(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_api_keys_rig ON api_keys(rig_id) WHERE rig_id IS NOT NULL;

-- Create rig telemetry table
CREATE TABLE rig_telemetry (
    telemetry_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rig_id UUID NOT NULL REFERENCES rigs(rig_id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    algorithm VARCHAR(50) NOT NULL,
    total_hashrate DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_power DOUBLE PRECISION NOT NULL DEFAULT 0,
    avg_temperature DOUBLE PRECISION NOT NULL DEFAULT 0,
    device_count INTEGER NOT NULL DEFAULT 0,
    shares_accepted BIGINT NOT NULL DEFAULT 0,
    shares_rejected BIGINT NOT NULL DEFAULT 0,
    pool_url VARCHAR(255) NOT NULL,
    profit_eur_day DOUBLE PRECISION,
    device_telemetry JSONB NOT NULL DEFAULT '[]'::jsonb
);

-- Create indexes for telemetry table
CREATE INDEX idx_telemetry_rig_timestamp ON rig_telemetry(rig_id, timestamp DESC);
CREATE INDEX idx_telemetry_timestamp ON rig_telemetry(timestamp DESC);
CREATE INDEX idx_telemetry_algorithm ON rig_telemetry(algorithm);

-- Create a partial index for recent telemetry (last 24 hours)
CREATE INDEX idx_telemetry_recent ON rig_telemetry(rig_id, timestamp DESC) 
WHERE timestamp > NOW() - INTERVAL '24 hours';

-- Create triggers to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply the trigger to relevant tables
CREATE TRIGGER users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER rigs_updated_at BEFORE UPDATE ON rigs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER api_keys_updated_at BEFORE UPDATE ON api_keys
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Create function to clean old telemetry data (older than 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_telemetry()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM rig_telemetry 
    WHERE timestamp < NOW() - INTERVAL '30 days';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Create initial admin user (password: "admin123" - CHANGE IN PRODUCTION!)
-- Password hash for "admin123" using Argon2
INSERT INTO users (email, password_hash) VALUES (
    'admin@bunker.local',
    '$argon2id$v=19$m=19456,t=2,p=1$VwYXxJKwlg4YLmRVKjxJRA$EBjFJLXgLs6aAb1u6qQ8cJh1oqhAyxqB1sQ8RjKjKqE'
);

-- Grant necessary permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO bunker;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO bunker;