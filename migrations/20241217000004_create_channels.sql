-- Migration: 20241217000004_create_channels
-- Description: Create channels table with support for text, voice, categories, and DMs
-- Author: PostgreSQL Schema Architect

-- Channel types enum
CREATE TYPE channel_type AS ENUM ('text', 'voice', 'category', 'dm', 'group_dm');

-- Channels table
CREATE TABLE channels (
    id BIGINT PRIMARY KEY,
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,  -- NULL for DMs
    name VARCHAR(100) NOT NULL,
    type channel_type NOT NULL DEFAULT 'text',
    topic TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    parent_id BIGINT REFERENCES channels(id) ON DELETE SET NULL,  -- Category reference
    nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    rate_limit_per_user INTEGER DEFAULT 0,  -- Slowmode in seconds
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER update_channels_updated_at
    BEFORE UPDATE ON channels
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE INDEX idx_channels_server_id ON channels(server_id);
CREATE INDEX idx_channels_parent_id ON channels(parent_id);
CREATE INDEX idx_channels_server_position ON channels(server_id, position);
