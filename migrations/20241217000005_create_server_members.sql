-- Migration: 20241217000005_create_server_members
-- Description: Create server_members junction table for user-server relationships
-- Author: PostgreSQL Schema Architect

-- Server members (junction table)
CREATE TABLE server_members (
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    nickname VARCHAR(32),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (server_id, user_id)
);

CREATE INDEX idx_server_members_user_id ON server_members(user_id);
CREATE INDEX idx_server_members_joined_at ON server_members(server_id, joined_at DESC);
