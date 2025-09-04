-- BUNKER MINER - Database Initialization Script
-- This script sets up the initial database schema for development
--
-- SECURITY NOTE: This script is for development only!
-- Production databases should use proper migration systems.

-- Set up database encoding and locale
SET client_encoding = 'UTF8';
SET timezone = 'UTC';

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- =============================================================================
-- DEVELOPMENT SCHEMA SETUP
-- =============================================================================

-- Create basic tables for stub service development
-- These are minimal schemas to support the stub services

-- Pool-related tables
CREATE TABLE IF NOT EXISTS pools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    fee_percentage DECIMAL(5,2) DEFAULT 1.0,
    minimum_payout DECIMAL(20,8) DEFAULT 0.1,
    payment_threshold DECIMAL(20,8) DEFAULT 1.0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Miner-related tables
CREATE TABLE IF NOT EXISTS miners (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    miner_id VARCHAR(255) UNIQUE NOT NULL,
    worker_name VARCHAR(255) NOT NULL,
    wallet_address VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Mining statistics tables
CREATE TABLE IF NOT EXISTS miner_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    miner_id UUID REFERENCES miners(id) ON DELETE CASCADE,
    hashrate_hs BIGINT DEFAULT 0,
    shares_submitted BIGINT DEFAULT 0,
    shares_accepted BIGINT DEFAULT 0,
    difficulty DECIMAL(20,2) DEFAULT 1000.0,
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    connected_since TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    status VARCHAR(50) DEFAULT 'offline',
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Payout information tables
CREATE TABLE IF NOT EXISTS payouts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    miner_id UUID REFERENCES miners(id) ON DELETE CASCADE,
    wallet_address VARCHAR(255) NOT NULL,
    pending_balance DECIMAL(20,8) DEFAULT 0.0,
    total_paid DECIMAL(20,8) DEFAULT 0.0,
    last_payment TIMESTAMP WITH TIME ZONE,
    payment_threshold DECIMAL(20,8) DEFAULT 1.0,
    auto_payout BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Block information tables
CREATE TABLE IF NOT EXISTS blocks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    height BIGINT NOT NULL UNIQUE,
    hash VARCHAR(64) NOT NULL UNIQUE,
    prev_hash VARCHAR(64) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    difficulty DECIMAL(20,2) NOT NULL,
    reward DECIMAL(20,8) NOT NULL,
    finder_miner_id UUID REFERENCES miners(id),
    confirmations INTEGER DEFAULT 0,
    status VARCHAR(50) DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Fleet management tables
CREATE TABLE IF NOT EXISTS fleet_connections (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id VARCHAR(255) UNIQUE NOT NULL,
    miner_id UUID REFERENCES miners(id) ON DELETE CASCADE,
    connected_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    telemetry_messages_received BIGINT DEFAULT 0,
    status VARCHAR(50) DEFAULT 'connected',
    client_info JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Telemetry data tables
CREATE TABLE IF NOT EXISTS telemetry (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    miner_id UUID REFERENCES miners(id) ON DELETE CASCADE,
    session_id VARCHAR(255),
    message_type VARCHAR(100) NOT NULL,
    data JSONB NOT NULL,
    received_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- =============================================================================
-- INDEXES FOR PERFORMANCE
-- =============================================================================

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_miners_miner_id ON miners(miner_id);
CREATE INDEX IF NOT EXISTS idx_miner_stats_miner_id ON miner_stats(miner_id);
CREATE INDEX IF NOT EXISTS idx_miner_stats_recorded_at ON miner_stats(recorded_at);
CREATE INDEX IF NOT EXISTS idx_payouts_miner_id ON payouts(miner_id);
CREATE INDEX IF NOT EXISTS idx_payouts_wallet_address ON payouts(wallet_address);
CREATE INDEX IF NOT EXISTS idx_blocks_height ON blocks(height);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);
CREATE INDEX IF NOT EXISTS idx_fleet_connections_session_id ON fleet_connections(session_id);
CREATE INDEX IF NOT EXISTS idx_fleet_connections_miner_id ON fleet_connections(miner_id);
CREATE INDEX IF NOT EXISTS idx_telemetry_miner_id ON telemetry(miner_id);
CREATE INDEX IF NOT EXISTS idx_telemetry_received_at ON telemetry(received_at);

-- =============================================================================
-- DEVELOPMENT DATA SEEDING
-- =============================================================================

-- Insert default pool
INSERT INTO pools (name, description, fee_percentage, minimum_payout, payment_threshold)
VALUES (
    'BUNKER Development Pool',
    'Local development pool for testing BUNKER MINER',
    1.0,
    0.1,
    1.0
) ON CONFLICT DO NOTHING;

-- Insert sample miners for development
INSERT INTO miners (miner_id, worker_name, wallet_address) VALUES
    ('miner_0001', 'alice_miner', '48abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12'),
    ('miner_0002', 'bob_farm', '48fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321fedcba09'),
    ('miner_0003', 'crypto_enthusiast', '48111111222222333333444444555555666666777777888888999999aaaaaabbbbbb'),
    ('miner_0004', 'mining_rig_01', '48ccccccddddddeeeeeeffffffgggggghhhhhiiiiijjjjjkkkkklllllmmmmmnnnnnn')
ON CONFLICT (miner_id) DO NOTHING;

-- Insert sample payout information
INSERT INTO payouts (miner_id, wallet_address, pending_balance, total_paid, payment_threshold, auto_payout)
SELECT 
    m.id,
    m.wallet_address,
    0.5 + (RANDOM() * 2.0)::DECIMAL(20,8),
    10.0 + (RANDOM() * 20.0)::DECIMAL(20,8),
    1.0,
    TRUE
FROM miners m
ON CONFLICT DO NOTHING;

-- =============================================================================
-- TRIGGERS AND FUNCTIONS
-- =============================================================================

-- Function to update updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at timestamps
CREATE TRIGGER update_pools_updated_at BEFORE UPDATE ON pools
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_miners_updated_at BEFORE UPDATE ON miners
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payouts_updated_at BEFORE UPDATE ON payouts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- PERMISSIONS AND SECURITY
-- =============================================================================

-- Grant appropriate permissions to the application user
GRANT CONNECT ON DATABASE bunker_miner TO bunker_admin;
GRANT USAGE ON SCHEMA public TO bunker_admin;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO bunker_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO bunker_admin;

-- =============================================================================
-- DEVELOPMENT UTILITIES
-- =============================================================================

-- Create a view for easy monitoring of miner status
CREATE OR REPLACE VIEW miner_overview AS
SELECT 
    m.miner_id,
    m.worker_name,
    ms.hashrate_hs,
    ms.shares_submitted,
    ms.shares_accepted,
    CASE 
        WHEN ms.shares_submitted > 0 THEN 
            ROUND((ms.shares_accepted::DECIMAL / ms.shares_submitted::DECIMAL) * 100, 2)
        ELSE 0
    END AS acceptance_rate_percent,
    ms.status,
    ms.last_seen,
    p.pending_balance,
    p.total_paid
FROM miners m
LEFT JOIN LATERAL (
    SELECT * FROM miner_stats 
    WHERE miner_id = m.id 
    ORDER BY recorded_at DESC 
    LIMIT 1
) ms ON true
LEFT JOIN payouts p ON p.miner_id = m.id;

-- Create a function to generate sample telemetry data (for testing)
CREATE OR REPLACE FUNCTION generate_sample_telemetry()
RETURNS void AS $$
DECLARE
    miner_record RECORD;
BEGIN
    FOR miner_record IN SELECT id FROM miners LOOP
        INSERT INTO telemetry (miner_id, message_type, data)
        VALUES (
            miner_record.id,
            'performance_report',
            jsonb_build_object(
                'hashrate', 1000 + (RANDOM() * 2000)::INT,
                'temperature', 60 + (RANDOM() * 20)::INT,
                'power_consumption', 200 + (RANDOM() * 100)::INT,
                'fan_speed', 1500 + (RANDOM() * 1000)::INT
            )
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Log successful initialization
INSERT INTO telemetry (miner_id, message_type, data)
SELECT 
    m.id,
    'system_event',
    jsonb_build_object(
        'event', 'database_initialized',
        'timestamp', NOW(),
        'version', '0.1.0-stub'
    )
FROM miners m
LIMIT 1;