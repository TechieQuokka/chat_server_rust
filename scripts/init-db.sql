-- ============================================
-- Chat Server Database Initialization
-- PostgreSQL 16
-- ============================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";      -- For text search
CREATE EXTENSION IF NOT EXISTS "btree_gin";    -- For GIN indexes

-- ============================================
-- Custom Types
-- ============================================

-- Channel types
DO $$ BEGIN
    CREATE TYPE channel_type AS ENUM (
        'text',
        'voice',
        'announcement',
        'stage',
        'forum'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- User status
DO $$ BEGIN
    CREATE TYPE user_status AS ENUM (
        'online',
        'idle',
        'dnd',
        'offline',
        'invisible'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Message types
DO $$ BEGIN
    CREATE TYPE message_type AS ENUM (
        'default',
        'recipient_add',
        'recipient_remove',
        'call',
        'channel_name_change',
        'channel_icon_change',
        'channel_pinned_message',
        'user_join',
        'guild_boost',
        'reply',
        'slash_command',
        'thread_created'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- ============================================
-- Users Table
-- ============================================
CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    discriminator VARCHAR(4) NOT NULL DEFAULT '0000',
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(32),
    avatar_hash VARCHAR(64),
    banner_hash VARCHAR(64),
    bio TEXT,
    status user_status DEFAULT 'offline',
    custom_status VARCHAR(128),
    locale VARCHAR(10) DEFAULT 'en-US',
    flags BIGINT DEFAULT 0,
    premium_type SMALLINT DEFAULT 0,
    mfa_enabled BOOLEAN DEFAULT FALSE,
    verified BOOLEAN DEFAULT FALSE,
    email_verified_at TIMESTAMPTZ,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,

    CONSTRAINT users_username_length CHECK (char_length(username) >= 2),
    CONSTRAINT users_discriminator_format CHECK (discriminator ~ '^\d{4}$')
);

-- Indexes for users
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NULL;

-- ============================================
-- Servers (Guilds) Table
-- ============================================
CREATE TABLE IF NOT EXISTS servers (
    id BIGINT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    icon_hash VARCHAR(64),
    banner_hash VARCHAR(64),
    splash_hash VARCHAR(64),
    discovery_splash_hash VARCHAR(64),
    region VARCHAR(32),
    afk_channel_id BIGINT,
    afk_timeout INTEGER DEFAULT 300,
    system_channel_id BIGINT,
    rules_channel_id BIGINT,
    public_updates_channel_id BIGINT,
    verification_level SMALLINT DEFAULT 0,
    default_message_notifications SMALLINT DEFAULT 0,
    explicit_content_filter SMALLINT DEFAULT 0,
    features TEXT[] DEFAULT '{}',
    mfa_level SMALLINT DEFAULT 0,
    premium_tier SMALLINT DEFAULT 0,
    premium_subscription_count INTEGER DEFAULT 0,
    preferred_locale VARCHAR(10) DEFAULT 'en-US',
    max_members INTEGER DEFAULT 500000,
    vanity_url_code VARCHAR(32) UNIQUE,
    nsfw_level SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,

    CONSTRAINT servers_name_length CHECK (char_length(name) >= 2)
);

-- Indexes for servers
CREATE INDEX IF NOT EXISTS idx_servers_owner_id ON servers(owner_id);
CREATE INDEX IF NOT EXISTS idx_servers_name ON servers USING gin(name gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_servers_created_at ON servers(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_servers_deleted_at ON servers(deleted_at) WHERE deleted_at IS NULL;

-- ============================================
-- Channels Table
-- ============================================
CREATE TABLE IF NOT EXISTS channels (
    id BIGINT PRIMARY KEY,
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    parent_id BIGINT REFERENCES channels(id) ON DELETE SET NULL,
    name VARCHAR(100) NOT NULL,
    topic TEXT,
    type channel_type NOT NULL DEFAULT 'text',
    position INTEGER NOT NULL DEFAULT 0,
    permission_overwrites JSONB DEFAULT '[]',
    nsfw BOOLEAN DEFAULT FALSE,
    rate_limit_per_user INTEGER DEFAULT 0,
    bitrate INTEGER,           -- For voice channels
    user_limit INTEGER,        -- For voice channels
    rtc_region VARCHAR(32),    -- For voice channels
    video_quality_mode SMALLINT DEFAULT 1,
    last_message_id BIGINT,
    last_pin_timestamp TIMESTAMPTZ,
    default_auto_archive_duration INTEGER DEFAULT 1440,
    default_thread_rate_limit_per_user INTEGER DEFAULT 0,
    flags INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,

    CONSTRAINT channels_name_length CHECK (char_length(name) >= 1)
);

-- Indexes for channels
CREATE INDEX IF NOT EXISTS idx_channels_server_id ON channels(server_id);
CREATE INDEX IF NOT EXISTS idx_channels_parent_id ON channels(parent_id);
CREATE INDEX IF NOT EXISTS idx_channels_type ON channels(type);
CREATE INDEX IF NOT EXISTS idx_channels_position ON channels(server_id, position);
CREATE INDEX IF NOT EXISTS idx_channels_deleted_at ON channels(deleted_at) WHERE deleted_at IS NULL;

-- ============================================
-- Messages Table
-- ============================================
CREATE TABLE IF NOT EXISTS messages (
    id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id BIGINT NOT NULL REFERENCES users(id),
    content TEXT,
    type message_type DEFAULT 'default',
    tts BOOLEAN DEFAULT FALSE,
    mention_everyone BOOLEAN DEFAULT FALSE,
    mentions BIGINT[] DEFAULT '{}',
    mention_roles BIGINT[] DEFAULT '{}',
    mention_channels BIGINT[] DEFAULT '{}',
    attachments JSONB DEFAULT '[]',
    embeds JSONB DEFAULT '[]',
    reactions JSONB DEFAULT '[]',
    pinned BOOLEAN DEFAULT FALSE,
    webhook_id BIGINT,
    activity JSONB,
    application JSONB,
    application_id BIGINT,
    message_reference JSONB,
    referenced_message_id BIGINT,
    flags INTEGER DEFAULT 0,
    thread_id BIGINT,
    components JSONB DEFAULT '[]',
    sticker_items JSONB DEFAULT '[]',
    position INTEGER,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

-- Indexes for messages (critical for performance)
CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id);
CREATE INDEX IF NOT EXISTS idx_messages_author_id ON messages(author_id);
CREATE INDEX IF NOT EXISTS idx_messages_channel_created ON messages(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_thread_id ON messages(thread_id) WHERE thread_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_messages_referenced ON messages(referenced_message_id) WHERE referenced_message_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_messages_content_search ON messages USING gin(to_tsvector('english', content));
CREATE INDEX IF NOT EXISTS idx_messages_pinned ON messages(channel_id, pinned) WHERE pinned = TRUE;
CREATE INDEX IF NOT EXISTS idx_messages_deleted_at ON messages(deleted_at) WHERE deleted_at IS NULL;

-- ============================================
-- Roles Table
-- ============================================
CREATE TABLE IF NOT EXISTS roles (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    color INTEGER DEFAULT 0,
    hoist BOOLEAN DEFAULT FALSE,
    icon_hash VARCHAR(64),
    unicode_emoji VARCHAR(32),
    position INTEGER NOT NULL DEFAULT 0,
    permissions BIGINT NOT NULL DEFAULT 0,
    managed BOOLEAN DEFAULT FALSE,
    mentionable BOOLEAN DEFAULT FALSE,
    tags JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for roles
CREATE INDEX IF NOT EXISTS idx_roles_server_id ON roles(server_id);
CREATE INDEX IF NOT EXISTS idx_roles_position ON roles(server_id, position);

-- ============================================
-- Server Members Table
-- ============================================
CREATE TABLE IF NOT EXISTS server_members (
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    nickname VARCHAR(32),
    avatar_hash VARCHAR(64),
    joined_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    premium_since TIMESTAMPTZ,
    deaf BOOLEAN DEFAULT FALSE,
    mute BOOLEAN DEFAULT FALSE,
    pending BOOLEAN DEFAULT FALSE,
    communication_disabled_until TIMESTAMPTZ,
    flags INTEGER DEFAULT 0,

    PRIMARY KEY (server_id, user_id)
);

-- Indexes for server_members
CREATE INDEX IF NOT EXISTS idx_server_members_user_id ON server_members(user_id);
CREATE INDEX IF NOT EXISTS idx_server_members_joined_at ON server_members(server_id, joined_at);

-- ============================================
-- Member Roles Junction Table
-- ============================================
CREATE TABLE IF NOT EXISTS member_roles (
    server_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,

    PRIMARY KEY (server_id, user_id, role_id),
    FOREIGN KEY (server_id, user_id) REFERENCES server_members(server_id, user_id) ON DELETE CASCADE
);

-- Indexes for member_roles
CREATE INDEX IF NOT EXISTS idx_member_roles_role_id ON member_roles(role_id);

-- ============================================
-- Invites Table
-- ============================================
CREATE TABLE IF NOT EXISTS invites (
    code VARCHAR(16) PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    inviter_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    target_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    target_type SMALLINT,
    max_age INTEGER DEFAULT 86400,
    max_uses INTEGER DEFAULT 0,
    uses INTEGER DEFAULT 0,
    temporary BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ
);

-- Indexes for invites
CREATE INDEX IF NOT EXISTS idx_invites_server_id ON invites(server_id);
CREATE INDEX IF NOT EXISTS idx_invites_inviter_id ON invites(inviter_id);
CREATE INDEX IF NOT EXISTS idx_invites_expires_at ON invites(expires_at) WHERE expires_at IS NOT NULL;

-- ============================================
-- Bans Table
-- ============================================
CREATE TABLE IF NOT EXISTS bans (
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT,
    banned_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (server_id, user_id)
);

-- ============================================
-- Audit Log Table
-- ============================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    target_id BIGINT,
    action_type INTEGER NOT NULL,
    changes JSONB DEFAULT '[]',
    options JSONB,
    reason TEXT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for audit_logs
CREATE INDEX IF NOT EXISTS idx_audit_logs_server_id ON audit_logs(server_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_target_id ON audit_logs(target_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action_type ON audit_logs(action_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(server_id, created_at DESC);

-- ============================================
-- Webhooks Table
-- ============================================
CREATE TABLE IF NOT EXISTS webhooks (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    name VARCHAR(80) NOT NULL,
    avatar_hash VARCHAR(64),
    token VARCHAR(68),
    application_id BIGINT,
    source_guild_id BIGINT,
    source_channel_id BIGINT,
    url TEXT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for webhooks
CREATE INDEX IF NOT EXISTS idx_webhooks_server_id ON webhooks(server_id);
CREATE INDEX IF NOT EXISTS idx_webhooks_channel_id ON webhooks(channel_id);

-- ============================================
-- Refresh Tokens Table
-- ============================================
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL UNIQUE,
    device_info JSONB,
    ip_address INET,
    user_agent TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMPTZ
);

-- Indexes for refresh_tokens
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_revoked_at ON refresh_tokens(revoked_at) WHERE revoked_at IS NULL;

-- ============================================
-- Sessions Table (for presence tracking)
-- ============================================
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id VARCHAR(64) NOT NULL UNIQUE,
    status user_status DEFAULT 'online',
    client_info JSONB,
    activities JSONB DEFAULT '[]',
    connected_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    last_heartbeat_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    disconnected_at TIMESTAMPTZ
);

-- Indexes for sessions
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(disconnected_at) WHERE disconnected_at IS NULL;

-- ============================================
-- Helper Functions
-- ============================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers for updated_at
DO $$
DECLARE
    t text;
BEGIN
    FOR t IN
        SELECT table_name
        FROM information_schema.columns
        WHERE column_name = 'updated_at'
        AND table_schema = 'public'
    LOOP
        EXECUTE format('
            DROP TRIGGER IF EXISTS update_%I_updated_at ON %I;
            CREATE TRIGGER update_%I_updated_at
            BEFORE UPDATE ON %I
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column();
        ', t, t, t, t);
    END LOOP;
END $$;

-- ============================================
-- Grant Permissions
-- ============================================
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO chat_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO chat_user;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO chat_user;

-- ============================================
-- Completion Message
-- ============================================
DO $$
BEGIN
    RAISE NOTICE 'Chat Server database initialization completed successfully!';
END $$;
