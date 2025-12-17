-- Migration: Create invites table
-- Description: Server invite links with usage limits and expiration

-- Server invites table
CREATE TABLE invites (
    code VARCHAR(10) PRIMARY KEY,  -- Short invite code (e.g., 'aBcD1234')
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    inviter_id BIGINT REFERENCES users(id) ON DELETE SET NULL,  -- NULL if inviter deleted account
    max_uses INTEGER NOT NULL DEFAULT 0,  -- 0 = unlimited uses
    uses INTEGER NOT NULL DEFAULT 0,  -- Current usage count
    max_age INTEGER NOT NULL DEFAULT 0,  -- Seconds until expiry, 0 = never expires
    temporary BOOLEAN NOT NULL DEFAULT FALSE,  -- Temporary membership (kicked when offline)
    expires_at TIMESTAMPTZ,  -- Computed expiration timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure code is valid length
    CONSTRAINT invites_code_length CHECK (char_length(code) >= 6),
    -- Ensure max_uses is non-negative
    CONSTRAINT invites_max_uses_non_negative CHECK (max_uses >= 0),
    -- Ensure uses is non-negative
    CONSTRAINT invites_uses_non_negative CHECK (uses >= 0),
    -- Ensure uses doesn't exceed max_uses (when max_uses > 0)
    CONSTRAINT invites_uses_within_limit CHECK (
        max_uses = 0 OR uses <= max_uses
    ),
    -- Ensure max_age is non-negative
    CONSTRAINT invites_max_age_non_negative CHECK (max_age >= 0)
);

-- Index for finding invites by server
CREATE INDEX idx_invites_server_id ON invites(server_id);

-- Index for finding invites by channel
CREATE INDEX idx_invites_channel_id ON invites(channel_id);

-- Index for finding invites created by a user
CREATE INDEX idx_invites_inviter_id ON invites(inviter_id)
    WHERE inviter_id IS NOT NULL;

-- Partial index for active (non-expired) invites
-- Useful for cleanup jobs and validation
CREATE INDEX idx_invites_expires_at ON invites(expires_at)
    WHERE expires_at IS NOT NULL;

-- Index for finding valid invites (not expired, not maxed out)
CREATE INDEX idx_invites_valid ON invites(code)
    WHERE (expires_at IS NULL OR expires_at > NOW())
    AND (max_uses = 0 OR uses < max_uses);

COMMENT ON TABLE invites IS 'Server invite links with usage tracking';
COMMENT ON COLUMN invites.code IS 'Short alphanumeric invite code';
COMMENT ON COLUMN invites.max_uses IS 'Maximum number of uses, 0 for unlimited';
COMMENT ON COLUMN invites.uses IS 'Current number of times this invite was used';
COMMENT ON COLUMN invites.max_age IS 'Seconds until expiration, 0 for never';
COMMENT ON COLUMN invites.temporary IS 'If true, members are kicked when they go offline';
COMMENT ON COLUMN invites.expires_at IS 'Pre-computed expiration timestamp for efficient queries';
