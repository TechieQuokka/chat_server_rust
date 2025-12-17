-- ============================================
-- Migration: Create User Sessions Table
-- Description: Enhanced session management for JWT refresh token handling
--              Supports device tracking, IP logging, and session expiry
-- ============================================

-- User sessions table for JWT refresh token management
-- Each session represents an authenticated device/client
CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash VARCHAR(255) NOT NULL,  -- SHA-256 hash of refresh token
    device_info TEXT,                          -- User agent or device description
    device_type VARCHAR(20),                   -- 'desktop', 'mobile', 'tablet', 'browser', 'bot'
    os_info VARCHAR(50),                       -- Operating system info
    ip_address INET,                           -- Client IP address
    location_info JSONB,                       -- Geo-location data if available
    expires_at TIMESTAMPTZ NOT NULL,           -- When this session expires
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,                    -- NULL if active, timestamp if revoked

    -- Ensure device_type is valid if provided
    CONSTRAINT user_sessions_device_type_check
        CHECK (device_type IS NULL OR device_type IN (
            'desktop', 'mobile', 'tablet', 'browser', 'bot', 'unknown'
        ))
);

-- Add table comments
COMMENT ON TABLE user_sessions IS
    'Tracks active user sessions for JWT refresh token management';
COMMENT ON COLUMN user_sessions.refresh_token_hash IS
    'SHA-256 hash of the refresh token (never store raw tokens)';
COMMENT ON COLUMN user_sessions.device_info IS
    'Raw user agent string or device description';
COMMENT ON COLUMN user_sessions.device_type IS
    'Normalized device category for filtering';
COMMENT ON COLUMN user_sessions.ip_address IS
    'Client IP address at session creation (PostgreSQL INET type)';
COMMENT ON COLUMN user_sessions.location_info IS
    'Optional geo-location data: {"country": "US", "city": "New York", ...}';
COMMENT ON COLUMN user_sessions.last_used_at IS
    'Updated each time the refresh token is used';
COMMENT ON COLUMN user_sessions.revoked_at IS
    'Set when session is explicitly revoked (logout, security event)';

-- Index for looking up all sessions for a user
-- Used for "manage sessions" UI and logout-all-devices
CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id
    ON user_sessions(user_id);

-- Index for cleaning up expired sessions
-- Background job can efficiently find sessions past expiry
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires
    ON user_sessions(expires_at)
    WHERE revoked_at IS NULL;

-- Index for token lookup during refresh
-- This is the hot path - must be fast
CREATE INDEX IF NOT EXISTS idx_user_sessions_token
    ON user_sessions(refresh_token_hash)
    WHERE revoked_at IS NULL;

-- Partial index for active sessions only
-- Excludes revoked sessions for faster active session queries
CREATE INDEX IF NOT EXISTS idx_user_sessions_active
    ON user_sessions(user_id, last_used_at DESC)
    WHERE revoked_at IS NULL;

-- Index for security monitoring - sessions by IP
-- Useful for detecting suspicious activity from same IP
CREATE INDEX IF NOT EXISTS idx_user_sessions_ip
    ON user_sessions(ip_address)
    WHERE ip_address IS NOT NULL;

-- ============================================
-- Helper function to clean up expired sessions
-- Should be called periodically by a background job
-- ============================================
CREATE OR REPLACE FUNCTION cleanup_expired_sessions()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM user_sessions
    WHERE expires_at < NOW()
       OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '30 days');

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_expired_sessions() IS
    'Removes expired sessions and old revoked sessions (>30 days). Returns count of deleted rows.';

-- ============================================
-- Helper function to revoke all sessions for a user
-- Useful for password changes, security events, or user request
-- ============================================
CREATE OR REPLACE FUNCTION revoke_user_sessions(
    p_user_id BIGINT,
    p_except_session_id UUID DEFAULT NULL
)
RETURNS INTEGER AS $$
DECLARE
    revoked_count INTEGER;
BEGIN
    UPDATE user_sessions
    SET revoked_at = NOW()
    WHERE user_id = p_user_id
      AND revoked_at IS NULL
      AND (p_except_session_id IS NULL OR id != p_except_session_id);

    GET DIAGNOSTICS revoked_count = ROW_COUNT;
    RETURN revoked_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION revoke_user_sessions(BIGINT, UUID) IS
    'Revokes all active sessions for a user, optionally keeping one session active. Returns count of revoked sessions.';

-- ============================================
-- Function to update last_used_at on token use
-- ============================================
CREATE OR REPLACE FUNCTION touch_session(p_session_id UUID)
RETURNS BOOLEAN AS $$
BEGIN
    UPDATE user_sessions
    SET last_used_at = NOW()
    WHERE id = p_session_id
      AND revoked_at IS NULL
      AND expires_at > NOW();

    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION touch_session(UUID) IS
    'Updates last_used_at for a session. Returns false if session not found or expired.';
