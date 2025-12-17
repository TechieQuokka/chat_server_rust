-- ============================================
-- Migration: Enhanced Audit Logs with ENUM Action Types
-- Description: Adds audit_action_type ENUM and improves audit_logs table
--              Provides better type safety and more descriptive action tracking
-- ============================================

-- Create ENUM type for audit actions
-- Using DO block to handle case where type already exists
DO $$ BEGIN
    CREATE TYPE audit_action_type AS ENUM (
        -- Guild/Server actions
        'GUILD_UPDATE',

        -- Channel actions
        'CHANNEL_CREATE',
        'CHANNEL_UPDATE',
        'CHANNEL_DELETE',
        'CHANNEL_OVERWRITE_CREATE',
        'CHANNEL_OVERWRITE_UPDATE',
        'CHANNEL_OVERWRITE_DELETE',

        -- Member moderation actions
        'MEMBER_KICK',
        'MEMBER_PRUNE',
        'MEMBER_BAN_ADD',
        'MEMBER_BAN_REMOVE',
        'MEMBER_UPDATE',
        'MEMBER_ROLE_UPDATE',

        -- Role management actions
        'ROLE_CREATE',
        'ROLE_UPDATE',
        'ROLE_DELETE',

        -- Invite actions
        'INVITE_CREATE',
        'INVITE_UPDATE',
        'INVITE_DELETE',

        -- Message actions
        'MESSAGE_PIN',
        'MESSAGE_UNPIN',
        'MESSAGE_DELETE',
        'MESSAGE_BULK_DELETE',

        -- Webhook actions
        'WEBHOOK_CREATE',
        'WEBHOOK_UPDATE',
        'WEBHOOK_DELETE',

        -- Emoji actions
        'EMOJI_CREATE',
        'EMOJI_UPDATE',
        'EMOJI_DELETE',

        -- Integration actions
        'INTEGRATION_CREATE',
        'INTEGRATION_UPDATE',
        'INTEGRATION_DELETE'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- Create enhanced audit logs table
-- Note: If audit_logs already exists, this creates audit_logs_v2
-- You may want to migrate data and rename tables in production
CREATE TABLE IF NOT EXISTS audit_logs_v2 (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,  -- Actor who performed action
    action_type audit_action_type NOT NULL,
    target_type VARCHAR(50),  -- 'user', 'role', 'channel', 'message', etc.
    target_id VARCHAR(50),    -- ID of the target (stored as string for flexibility with different ID types)
    changes JSONB,            -- JSON object with before/after values
    reason TEXT,              -- Optional reason provided by moderator
    extra_data JSONB,         -- Additional context (message count for bulk delete, etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add table comments
COMMENT ON TABLE audit_logs_v2 IS
    'Audit log for tracking moderation and administrative actions in servers';
COMMENT ON COLUMN audit_logs_v2.user_id IS
    'The user who performed the action (NULL if system action or user deleted)';
COMMENT ON COLUMN audit_logs_v2.action_type IS
    'Type of action performed, using audit_action_type ENUM';
COMMENT ON COLUMN audit_logs_v2.target_type IS
    'Type of entity that was affected (user, role, channel, message, etc.)';
COMMENT ON COLUMN audit_logs_v2.target_id IS
    'ID of the affected entity (string for flexibility with different ID types)';
COMMENT ON COLUMN audit_logs_v2.changes IS
    'JSON object documenting what changed: {"key": {"old": value, "new": value}}';
COMMENT ON COLUMN audit_logs_v2.reason IS
    'Optional reason provided by the moderator for this action';

-- Primary index: Server + time for listing audit logs chronologically
-- Most common query pattern: "Show me recent audit logs for this server"
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_server_time
    ON audit_logs_v2(server_id, created_at DESC);

-- Index for filtering by actor (who did this?)
-- Partial index excluding NULL for efficiency
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_user
    ON audit_logs_v2(user_id)
    WHERE user_id IS NOT NULL;

-- Composite index for filtering by action type within a server
-- "Show me all role changes in this server"
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_action
    ON audit_logs_v2(server_id, action_type);

-- Index for finding all actions affecting a specific target
-- "Show me all actions taken against this user/channel/role"
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_target
    ON audit_logs_v2(target_type, target_id);

-- BRIN index for time-series queries on large tables
-- Efficient for range scans on created_at when table grows large
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_created_brin
    ON audit_logs_v2 USING BRIN(created_at)
    WITH (pages_per_range = 128);

-- GIN index for searching within changes JSONB
-- Enables queries like "find all logs where channel name was changed"
CREATE INDEX IF NOT EXISTS idx_audit_logs_v2_changes
    ON audit_logs_v2 USING GIN(changes jsonb_path_ops);

-- ============================================
-- Helper function to map old integer action_type to new ENUM
-- Useful for migration from existing audit_logs table
-- ============================================
CREATE OR REPLACE FUNCTION map_audit_action_int_to_enum(action_int INTEGER)
RETURNS audit_action_type AS $$
BEGIN
    RETURN CASE action_int
        WHEN 1 THEN 'GUILD_UPDATE'::audit_action_type
        WHEN 10 THEN 'CHANNEL_CREATE'::audit_action_type
        WHEN 11 THEN 'CHANNEL_UPDATE'::audit_action_type
        WHEN 12 THEN 'CHANNEL_DELETE'::audit_action_type
        WHEN 13 THEN 'CHANNEL_OVERWRITE_CREATE'::audit_action_type
        WHEN 14 THEN 'CHANNEL_OVERWRITE_UPDATE'::audit_action_type
        WHEN 15 THEN 'CHANNEL_OVERWRITE_DELETE'::audit_action_type
        WHEN 20 THEN 'MEMBER_KICK'::audit_action_type
        WHEN 21 THEN 'MEMBER_PRUNE'::audit_action_type
        WHEN 22 THEN 'MEMBER_BAN_ADD'::audit_action_type
        WHEN 23 THEN 'MEMBER_BAN_REMOVE'::audit_action_type
        WHEN 24 THEN 'MEMBER_UPDATE'::audit_action_type
        WHEN 25 THEN 'MEMBER_ROLE_UPDATE'::audit_action_type
        WHEN 30 THEN 'ROLE_CREATE'::audit_action_type
        WHEN 31 THEN 'ROLE_UPDATE'::audit_action_type
        WHEN 32 THEN 'ROLE_DELETE'::audit_action_type
        WHEN 40 THEN 'INVITE_CREATE'::audit_action_type
        WHEN 41 THEN 'INVITE_UPDATE'::audit_action_type
        WHEN 42 THEN 'INVITE_DELETE'::audit_action_type
        WHEN 74 THEN 'MESSAGE_PIN'::audit_action_type
        WHEN 75 THEN 'MESSAGE_UNPIN'::audit_action_type
        WHEN 72 THEN 'MESSAGE_DELETE'::audit_action_type
        WHEN 73 THEN 'MESSAGE_BULK_DELETE'::audit_action_type
        ELSE 'GUILD_UPDATE'::audit_action_type  -- Default fallback
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
