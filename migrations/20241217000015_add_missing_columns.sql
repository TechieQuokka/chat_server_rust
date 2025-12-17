-- ============================================
-- Migration: Add Missing Columns for Soft Delete and Voice State
-- Description: Adds deleted_at columns for soft delete support,
--              deaf/mute columns for voice state tracking
-- ============================================

-- ============================================
-- Add deleted_at columns for soft delete support
-- ============================================

-- Users table soft delete
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN users.deleted_at IS
    'Soft delete timestamp. NULL means the user is active.';

-- Servers table soft delete
ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN servers.deleted_at IS
    'Soft delete timestamp. NULL means the server is active.';

-- Channels table soft delete
ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN channels.deleted_at IS
    'Soft delete timestamp. NULL means the channel is active.';

-- Messages table soft delete
ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN messages.deleted_at IS
    'Soft delete timestamp. NULL means the message is active.';

-- Roles table soft delete (for consistency)
ALTER TABLE roles
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN roles.deleted_at IS
    'Soft delete timestamp. NULL means the role is active.';

-- ============================================
-- Add voice state columns to server_members
-- ============================================

ALTER TABLE server_members
    ADD COLUMN IF NOT EXISTS deaf BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS mute BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS voice_channel_id BIGINT NULL REFERENCES channels(id) ON DELETE SET NULL;

COMMENT ON COLUMN server_members.deaf IS
    'Whether the member is server-deafened in voice channels';
COMMENT ON COLUMN server_members.mute IS
    'Whether the member is server-muted in voice channels';
COMMENT ON COLUMN server_members.voice_channel_id IS
    'Current voice channel the member is connected to (NULL if not in voice)';

-- ============================================
-- Create audit_logs table (alias for audit_logs_v2)
-- This ensures the performance indexes can reference audit_logs
-- ============================================

-- Create a view that aliases audit_logs_v2 as audit_logs
-- This allows both names to work
CREATE OR REPLACE VIEW audit_logs AS
    SELECT * FROM audit_logs_v2;

-- ============================================
-- Add indexes for soft delete queries
-- ============================================

-- Index for active users (not deleted)
CREATE INDEX IF NOT EXISTS idx_users_active
    ON users(id)
    WHERE deleted_at IS NULL;

-- Index for active servers (not deleted)
CREATE INDEX IF NOT EXISTS idx_servers_active
    ON servers(id)
    WHERE deleted_at IS NULL;

-- Index for active channels (not deleted)
CREATE INDEX IF NOT EXISTS idx_channels_active
    ON channels(server_id)
    WHERE deleted_at IS NULL;

-- Index for active messages (not deleted)
CREATE INDEX IF NOT EXISTS idx_messages_active
    ON messages(channel_id, created_at DESC)
    WHERE deleted_at IS NULL;

-- Index for voice channel participants
CREATE INDEX IF NOT EXISTS idx_server_members_voice
    ON server_members(voice_channel_id)
    WHERE voice_channel_id IS NOT NULL;

-- ============================================
-- Update statistics
-- ============================================

ANALYZE users;
ANALYZE servers;
ANALYZE channels;
ANALYZE messages;
ANALYZE roles;
ANALYZE server_members;
