-- Migration: 20241217000003_create_roles
-- Description: Create roles table with 64-bit permission flags for flexible RBAC
-- Author: PostgreSQL Schema Architect

-- Roles table
CREATE TABLE roles (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    permissions BIGINT NOT NULL DEFAULT 0,  -- 64-bit permission flags
    position INTEGER NOT NULL DEFAULT 0,
    color INTEGER,  -- RGB color value
    hoist BOOLEAN NOT NULL DEFAULT FALSE,
    mentionable BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER update_roles_updated_at
    BEFORE UPDATE ON roles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE INDEX idx_roles_server_id ON roles(server_id);
CREATE INDEX idx_roles_server_position ON roles(server_id, position);
