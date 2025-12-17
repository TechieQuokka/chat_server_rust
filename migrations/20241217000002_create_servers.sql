-- Migration: 20241217000002_create_servers
-- Description: Create servers (guilds) table for multi-tenant architecture
-- Author: PostgreSQL Schema Architect

-- Servers (Guilds) table
CREATE TABLE servers (
    id BIGINT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    owner_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    icon_url TEXT,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER update_servers_updated_at
    BEFORE UPDATE ON servers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE INDEX idx_servers_owner_id ON servers(owner_id);
CREATE INDEX idx_servers_name ON servers(name);
