-- ============================================
-- Migration: Performance Indexes and Optimizations
-- Description: Additional indexes for common query patterns,
--              full-text search, and partial indexes for hot paths
-- ============================================

-- ============================================
-- Full-Text Search Indexes
-- ============================================

-- Full-text search on message content using GIN
-- Supports English language stemming and stop words
-- Note: If idx_messages_content_search already exists (from init-db.sql),
-- this will be skipped due to IF NOT EXISTS
CREATE INDEX IF NOT EXISTS idx_messages_content_fts
    ON messages USING GIN (to_tsvector('english', content))
    WHERE deleted_at IS NULL AND content IS NOT NULL;

COMMENT ON INDEX idx_messages_content_fts IS
    'Full-text search index for message content using English stemming';

-- Full-text search on server names and descriptions
CREATE INDEX IF NOT EXISTS idx_servers_search
    ON servers USING GIN (
        to_tsvector('english', COALESCE(name, '') || ' ' || COALESCE(description, ''))
    )
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_servers_search IS
    'Full-text search index for server name and description';

-- Full-text search on user profiles
CREATE INDEX IF NOT EXISTS idx_users_search
    ON users USING GIN (
        to_tsvector('english', COALESCE(username, '') || ' ' || COALESCE(display_name, '') || ' ' || COALESCE(bio, ''))
    )
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_users_search IS
    'Full-text search index for username, display name, and bio';

-- ============================================
-- Partial Indexes for Common Query Patterns
-- ============================================

-- Pinned messages per channel (frequently accessed, small subset)
CREATE INDEX IF NOT EXISTS idx_messages_pinned_per_channel
    ON messages(channel_id, created_at DESC)
    WHERE pinned = TRUE AND deleted_at IS NULL;

COMMENT ON INDEX idx_messages_pinned_per_channel IS
    'Fast lookup of pinned messages in a channel, sorted by date';

-- Active (non-expired, not maxed out) invites per server
CREATE INDEX IF NOT EXISTS idx_invites_active
    ON invites(server_id)
    WHERE (expires_at IS NULL OR expires_at > NOW())
      AND (max_uses = 0 OR uses < max_uses);

COMMENT ON INDEX idx_invites_active IS
    'Fast lookup of active invites for a server';

-- Unread messages optimization (messages after a certain point)
-- This index supports efficient "load messages since last read" queries
CREATE INDEX IF NOT EXISTS idx_messages_channel_id_created
    ON messages(channel_id, id)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_messages_channel_id_created IS
    'Supports cursor-based pagination and unread message queries';

-- Active voice channel participants
-- Useful for voice channel presence tracking
CREATE INDEX IF NOT EXISTS idx_server_members_voice_active
    ON server_members(server_id)
    WHERE deaf = FALSE AND mute = FALSE;

-- ============================================
-- Covering Indexes for Hot Paths
-- ============================================

-- Message list query covering index
-- Includes common columns to avoid heap fetches
CREATE INDEX IF NOT EXISTS idx_messages_list_covering
    ON messages(channel_id, created_at DESC)
    INCLUDE (author_id, content, type, pinned, edited_at)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_messages_list_covering IS
    'Covering index for message list queries - avoids heap fetches for common columns';

-- Channel list covering index
CREATE INDEX IF NOT EXISTS idx_channels_list_covering
    ON channels(server_id, position)
    INCLUDE (name, type, parent_id, topic)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_channels_list_covering IS
    'Covering index for channel list queries - includes common display columns';

-- Role list covering index
CREATE INDEX IF NOT EXISTS idx_roles_list_covering
    ON roles(server_id, position DESC)
    INCLUDE (name, color, permissions, hoist, mentionable);

COMMENT ON INDEX idx_roles_list_covering IS
    'Covering index for role list queries - includes display and permission columns';

-- ============================================
-- BRIN Indexes for Time-Series Data
-- ============================================

-- BRIN index for messages by creation time
-- Extremely efficient for large message tables with sequential inserts
CREATE INDEX IF NOT EXISTS idx_messages_created_brin
    ON messages USING BRIN(created_at)
    WITH (pages_per_range = 128);

COMMENT ON INDEX idx_messages_created_brin IS
    'BRIN index for efficient time-range queries on messages';

-- BRIN index for audit logs
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_brin
    ON audit_logs USING BRIN(created_at)
    WITH (pages_per_range = 128);

-- ============================================
-- Composite Indexes for Complex Queries
-- ============================================

-- Server member lookup with role filtering
CREATE INDEX IF NOT EXISTS idx_member_roles_lookup
    ON member_roles(server_id, role_id, user_id);

COMMENT ON INDEX idx_member_roles_lookup IS
    'Optimizes queries finding members with specific roles in a server';

-- User's servers with join date
CREATE INDEX IF NOT EXISTS idx_server_members_user_servers
    ON server_members(user_id, joined_at DESC);

COMMENT ON INDEX idx_server_members_user_servers IS
    'Fast lookup of all servers a user belongs to, sorted by join date';

-- ============================================
-- Expression Indexes
-- ============================================

-- Case-insensitive username lookup
CREATE INDEX IF NOT EXISTS idx_users_username_lower
    ON users(LOWER(username))
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_users_username_lower IS
    'Case-insensitive username search';

-- Case-insensitive email lookup (if not already handled by unique constraint)
CREATE INDEX IF NOT EXISTS idx_users_email_lower
    ON users(LOWER(email))
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_users_email_lower IS
    'Case-insensitive email lookup';

-- ============================================
-- Analyze Tables for Query Planner
-- ============================================

-- Update statistics for better query planning
-- Run after bulk data loads or periodically
ANALYZE users;
ANALYZE servers;
ANALYZE channels;
ANALYZE messages;
ANALYZE roles;
ANALYZE server_members;
ANALYZE member_roles;
ANALYZE invites;
ANALYZE audit_logs;

-- ============================================
-- Helpful Comments for Developers
-- ============================================

COMMENT ON INDEX idx_messages_content_fts IS
    'Usage: SELECT * FROM messages WHERE to_tsvector(''english'', content) @@ plainto_tsquery(''english'', ''search terms'')';

-- ============================================
-- Optional: Create extension for additional index types
-- ============================================

-- pg_trgm for similarity search (already in init-db.sql)
-- CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram index for fuzzy username matching
CREATE INDEX IF NOT EXISTS idx_users_username_trgm
    ON users USING GIN(username gin_trgm_ops)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_users_username_trgm IS
    'Trigram index for fuzzy username search. Usage: WHERE username % ''searchterm''';

-- Trigram index for fuzzy server name matching
CREATE INDEX IF NOT EXISTS idx_servers_name_trgm
    ON servers USING GIN(name gin_trgm_ops)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_servers_name_trgm IS
    'Trigram index for fuzzy server name search. Usage: WHERE name % ''searchterm''';
