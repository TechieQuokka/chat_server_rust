-- ============================================
-- Migration: Create Channel Permission Overwrites Table
-- Description: Normalized table for channel-specific permission overrides
--              Replaces JSONB permission_overwrites column for better querying
-- ============================================

-- Channel permission overwrites table
-- Can target either a role or a specific user (member)
-- This provides more granular control than the JSONB column in channels
CREATE TABLE IF NOT EXISTS channel_permission_overwrites (
    id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    target_type VARCHAR(10) NOT NULL,
    target_id BIGINT NOT NULL,  -- role_id or user_id depending on target_type
    allow BIGINT NOT NULL DEFAULT 0,  -- Allowed permissions (64-bit bitfield flags)
    deny BIGINT NOT NULL DEFAULT 0,   -- Denied permissions (64-bit bitfield flags)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure target_type is valid
    CONSTRAINT channel_overwrites_target_type_check
        CHECK (target_type IN ('role', 'member')),

    -- Ensure unique combination of channel + target_type + target_id
    -- A role or member can only have one overwrite per channel
    CONSTRAINT channel_overwrites_unique_target
        UNIQUE (channel_id, target_type, target_id),

    -- Ensure allow and deny don't have overlapping bits
    -- (a permission can't be both allowed and denied)
    CONSTRAINT channel_overwrites_no_overlap
        CHECK ((allow & deny) = 0)
);

-- Add table comment
COMMENT ON TABLE channel_permission_overwrites IS
    'Channel-specific permission overrides for roles and members';
COMMENT ON COLUMN channel_permission_overwrites.target_type IS
    'Type of target: role or member';
COMMENT ON COLUMN channel_permission_overwrites.target_id IS
    'ID of the role or user being overridden';
COMMENT ON COLUMN channel_permission_overwrites.allow IS
    '64-bit bitfield of explicitly allowed permissions';
COMMENT ON COLUMN channel_permission_overwrites.deny IS
    '64-bit bitfield of explicitly denied permissions';

-- Index for looking up all overwrites for a channel (common operation)
-- Used when loading channel data or calculating effective permissions
CREATE INDEX IF NOT EXISTS idx_channel_overwrites_channel
    ON channel_permission_overwrites(channel_id);

-- Composite index for looking up overwrites by target
-- Useful for finding all channels where a specific role/user has overrides
CREATE INDEX IF NOT EXISTS idx_channel_overwrites_target
    ON channel_permission_overwrites(target_type, target_id);

-- Index for finding all overwrites for a specific role across all channels
-- Useful when a role is deleted or modified
CREATE INDEX IF NOT EXISTS idx_channel_overwrites_role
    ON channel_permission_overwrites(target_id)
    WHERE target_type = 'role';

-- Index for finding all overwrites for a specific member across all channels
-- Useful when checking a user's permissions or when they leave
CREATE INDEX IF NOT EXISTS idx_channel_overwrites_member
    ON channel_permission_overwrites(target_id)
    WHERE target_type = 'member';

-- Trigger for automatic updated_at
CREATE OR REPLACE FUNCTION update_channel_overwrites_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_channel_overwrites_updated_at
    ON channel_permission_overwrites;
CREATE TRIGGER trigger_channel_overwrites_updated_at
    BEFORE UPDATE ON channel_permission_overwrites
    FOR EACH ROW
    EXECUTE FUNCTION update_channel_overwrites_updated_at();
