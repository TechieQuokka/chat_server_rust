-- ============================================
-- Chat Server v3 - Development Seed Data
-- ============================================
-- This script creates test data for local development
-- Run after migrations: psql -U chat_user -d chat_db -f seed-data.sql

BEGIN;

-- ============================================
-- 1. Test Users
-- ============================================
-- Password for all test users: "TestPassword123!"
-- Argon2id hash generated with default params

INSERT INTO users (id, username, email, password_hash, display_name, avatar_url, status, created_at) VALUES
  (
    1,
    'admin',
    'admin@chatserver.local',
    '$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG',
    'System Admin',
    NULL,
    'online',
    NOW() - INTERVAL '30 days'
  ),
  (
    2,
    'testuser1',
    'user1@chatserver.local',
    '$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG',
    'Test User One',
    NULL,
    'online',
    NOW() - INTERVAL '25 days'
  ),
  (
    3,
    'testuser2',
    'user2@chatserver.local',
    '$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG',
    'Test User Two',
    NULL,
    'idle',
    NOW() - INTERVAL '20 days'
  ),
  (
    4,
    'testuser3',
    'user3@chatserver.local',
    '$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG',
    'Test User Three',
    NULL,
    'dnd',
    NOW() - INTERVAL '15 days'
  ),
  (
    5,
    'bot_user',
    'bot@chatserver.local',
    '$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG',
    'Chat Bot',
    NULL,
    'online',
    NOW() - INTERVAL '30 days'
  )
ON CONFLICT (id) DO NOTHING;

-- Reset sequence
SELECT setval('users_id_seq', (SELECT MAX(id) FROM users));

-- ============================================
-- 2. Test Servers (Guilds)
-- ============================================

INSERT INTO servers (id, name, owner_id, icon_url, description, created_at) VALUES
  (
    1,
    'Development Server',
    1,
    NULL,
    'Main server for development and testing',
    NOW() - INTERVAL '30 days'
  ),
  (
    2,
    'Gaming Hub',
    2,
    NULL,
    'A place for gamers to hang out',
    NOW() - INTERVAL '20 days'
  ),
  (
    3,
    'Study Group',
    3,
    NULL,
    'Collaborative learning community',
    NOW() - INTERVAL '10 days'
  )
ON CONFLICT (id) DO NOTHING;

SELECT setval('servers_id_seq', (SELECT MAX(id) FROM servers));

-- ============================================
-- 3. Roles
-- ============================================
-- Permission bits (64-bit flags, Discord-compatible):
-- VIEW_CHANNEL = 1 << 10 = 1024
-- SEND_MESSAGES = 1 << 11 = 2048
-- MANAGE_MESSAGES = 1 << 13 = 8192
-- ADMINISTRATOR = 1 << 3 = 8
-- MANAGE_CHANNELS = 1 << 4 = 16
-- MANAGE_SERVER = 1 << 5 = 32
-- All permissions = 8589934591

INSERT INTO roles (id, server_id, name, permissions, position, color, hoist, mentionable, created_at) VALUES
  -- Development Server Roles
  (1, 1, '@everyone', 1024 | 2048, 0, NULL, false, false, NOW() - INTERVAL '30 days'),
  (2, 1, 'Admin', 8589934591, 3, 15158332, true, true, NOW() - INTERVAL '30 days'),
  (3, 1, 'Moderator', 1024 | 2048 | 8192 | 16, 2, 3447003, true, true, NOW() - INTERVAL '30 days'),
  (4, 1, 'Member', 1024 | 2048, 1, 10181046, false, false, NOW() - INTERVAL '30 days'),

  -- Gaming Hub Roles
  (5, 2, '@everyone', 1024 | 2048, 0, NULL, false, false, NOW() - INTERVAL '20 days'),
  (6, 2, 'Server Owner', 8589934591, 2, 15844367, true, true, NOW() - INTERVAL '20 days'),
  (7, 2, 'Gamer', 1024 | 2048, 1, 1752220, false, true, NOW() - INTERVAL '20 days'),

  -- Study Group Roles
  (8, 3, '@everyone', 1024 | 2048, 0, NULL, false, false, NOW() - INTERVAL '10 days'),
  (9, 3, 'Teacher', 8589934591, 2, 15105570, true, true, NOW() - INTERVAL '10 days'),
  (10, 3, 'Student', 1024 | 2048, 1, 3066993, false, false, NOW() - INTERVAL '10 days')
ON CONFLICT (id) DO NOTHING;

SELECT setval('roles_id_seq', (SELECT MAX(id) FROM roles));

-- ============================================
-- 4. Channels
-- ============================================
-- Channel types: 'text' = 0, 'voice' = 2, 'category' = 4

INSERT INTO channels (id, server_id, name, type, topic, position, parent_id, created_at) VALUES
  -- Development Server Channels
  (1, 1, 'Text Channels', 'category', NULL, 0, NULL, NOW() - INTERVAL '30 days'),
  (2, 1, 'general', 'text', 'General discussion', 1, 1, NOW() - INTERVAL '30 days'),
  (3, 1, 'random', 'text', 'Off-topic conversations', 2, 1, NOW() - INTERVAL '30 days'),
  (4, 1, 'announcements', 'text', 'Important announcements', 3, 1, NOW() - INTERVAL '30 days'),
  (5, 1, 'Voice Channels', 'category', NULL, 4, NULL, NOW() - INTERVAL '30 days'),
  (6, 1, 'General Voice', 'voice', NULL, 5, 5, NOW() - INTERVAL '30 days'),
  (7, 1, 'AFK', 'voice', NULL, 6, 5, NOW() - INTERVAL '30 days'),

  -- Gaming Hub Channels
  (8, 2, 'Lobby', 'category', NULL, 0, NULL, NOW() - INTERVAL '20 days'),
  (9, 2, 'welcome', 'text', 'Welcome new members!', 1, 8, NOW() - INTERVAL '20 days'),
  (10, 2, 'game-chat', 'text', 'Talk about games', 2, 8, NOW() - INTERVAL '20 days'),
  (11, 2, 'Voice Rooms', 'category', NULL, 3, NULL, NOW() - INTERVAL '20 days'),
  (12, 2, 'Gaming Voice', 'voice', NULL, 4, 11, NOW() - INTERVAL '20 days'),

  -- Study Group Channels
  (13, 3, 'Study Rooms', 'category', NULL, 0, NULL, NOW() - INTERVAL '10 days'),
  (14, 3, 'math', 'text', 'Mathematics discussion', 1, 13, NOW() - INTERVAL '10 days'),
  (15, 3, 'science', 'text', 'Science topics', 2, 13, NOW() - INTERVAL '10 days'),
  (16, 3, 'resources', 'text', 'Share learning resources', 3, 13, NOW() - INTERVAL '10 days')
ON CONFLICT (id) DO NOTHING;

SELECT setval('channels_id_seq', (SELECT MAX(id) FROM channels));

-- ============================================
-- 5. Server Members
-- ============================================

INSERT INTO server_members (server_id, user_id, nickname, joined_at) VALUES
  -- Development Server Members
  (1, 1, 'Admin', NOW() - INTERVAL '30 days'),
  (1, 2, 'Tester One', NOW() - INTERVAL '25 days'),
  (1, 3, NULL, NOW() - INTERVAL '20 days'),
  (1, 4, NULL, NOW() - INTERVAL '15 days'),
  (1, 5, 'Bot', NOW() - INTERVAL '30 days'),

  -- Gaming Hub Members
  (2, 2, 'ProGamer', NOW() - INTERVAL '20 days'),
  (2, 3, 'CasualPlayer', NOW() - INTERVAL '18 days'),
  (2, 4, NULL, NOW() - INTERVAL '15 days'),

  -- Study Group Members
  (3, 3, 'Professor', NOW() - INTERVAL '10 days'),
  (3, 4, 'Student A', NOW() - INTERVAL '8 days'),
  (3, 1, NULL, NOW() - INTERVAL '5 days')
ON CONFLICT (server_id, user_id) DO NOTHING;

-- ============================================
-- 6. Member Roles Assignment
-- ============================================

INSERT INTO member_roles (server_id, user_id, role_id) VALUES
  -- Development Server Role Assignments
  (1, 1, 2),  -- admin gets Admin role
  (1, 2, 3),  -- testuser1 gets Moderator role
  (1, 3, 4),  -- testuser2 gets Member role
  (1, 4, 4),  -- testuser3 gets Member role
  (1, 5, 4),  -- bot gets Member role

  -- Gaming Hub Role Assignments
  (2, 2, 6),  -- testuser1 is Server Owner
  (2, 3, 7),  -- testuser2 gets Gamer role
  (2, 4, 7),  -- testuser3 gets Gamer role

  -- Study Group Role Assignments
  (3, 3, 9),  -- testuser2 is Teacher
  (3, 4, 10), -- testuser3 is Student
  (3, 1, 10)  -- admin joins as Student
ON CONFLICT (server_id, user_id, role_id) DO NOTHING;

-- ============================================
-- 7. Sample Messages
-- ============================================

INSERT INTO messages (id, channel_id, author_id, content, message_type, created_at) VALUES
  -- General channel messages
  (1, 2, 1, 'Welcome to the Development Server! This is our main hub for testing.', 'default', NOW() - INTERVAL '29 days'),
  (2, 2, 2, 'Thanks for setting this up! Looking forward to testing.', 'default', NOW() - INTERVAL '28 days'),
  (3, 2, 3, 'Hello everyone! Excited to be here.', 'default', NOW() - INTERVAL '27 days'),
  (4, 2, 4, 'Hey! When do we start testing the new features?', 'default', NOW() - INTERVAL '26 days'),
  (5, 2, 1, 'We can start testing now. Feel free to explore all channels!', 'default', NOW() - INTERVAL '25 days'),
  (6, 2, 5, 'Beep boop! I am a bot and I am here to help.', 'default', NOW() - INTERVAL '24 days'),

  -- Random channel messages
  (7, 3, 2, 'Anyone up for some games later?', 'default', NOW() - INTERVAL '20 days'),
  (8, 3, 3, 'Sure! What do you want to play?', 'default', NOW() - INTERVAL '20 days' + INTERVAL '1 hour'),
  (9, 3, 4, 'Count me in!', 'default', NOW() - INTERVAL '20 days' + INTERVAL '2 hours'),

  -- Announcements
  (10, 4, 1, '**Important Update**\n\nWe are rolling out new features this week. Please report any bugs you find!', 'default', NOW() - INTERVAL '15 days'),

  -- Gaming Hub messages
  (11, 9, 2, 'Welcome to Gaming Hub! Introduce yourself here.', 'default', NOW() - INTERVAL '19 days'),
  (12, 10, 3, 'Anyone playing the new RPG?', 'default', NOW() - INTERVAL '18 days'),
  (13, 10, 4, 'Yes! It is amazing. The graphics are incredible.', 'default', NOW() - INTERVAL '17 days'),

  -- Study Group messages
  (14, 14, 3, 'Today we will cover calculus fundamentals.', 'default', NOW() - INTERVAL '9 days'),
  (15, 14, 4, 'I have a question about derivatives.', 'default', NOW() - INTERVAL '8 days'),
  (16, 15, 3, 'Let us discuss quantum physics basics.', 'default', NOW() - INTERVAL '7 days'),
  (17, 16, 4, 'Here is a great resource: https://example.com/learning', 'default', NOW() - INTERVAL '6 days')
ON CONFLICT (id) DO NOTHING;

SELECT setval('messages_id_seq', (SELECT MAX(id) FROM messages));

-- ============================================
-- 8. Message Reactions
-- ============================================

INSERT INTO message_reactions (message_id, user_id, emoji) VALUES
  (1, 2, '👋'),
  (1, 3, '👋'),
  (1, 4, '🎉'),
  (2, 1, '👍'),
  (5, 2, '✅'),
  (5, 3, '✅'),
  (10, 2, '📢'),
  (10, 3, '📢'),
  (10, 4, '📢'),
  (11, 3, '👋'),
  (14, 4, '📚'),
  (17, 3, '❤️')
ON CONFLICT (message_id, user_id, emoji) DO NOTHING;

-- ============================================
-- 9. Invites
-- ============================================

INSERT INTO invites (code, server_id, channel_id, inviter_id, max_uses, uses, expires_at, created_at) VALUES
  ('devserver', 1, 2, 1, 0, 5, NULL, NOW() - INTERVAL '25 days'),
  ('gaming123', 2, 9, 2, 100, 2, NOW() + INTERVAL '7 days', NOW() - INTERVAL '15 days'),
  ('study2024', 3, 14, 3, 50, 1, NOW() + INTERVAL '30 days', NOW() - INTERVAL '5 days')
ON CONFLICT (code) DO NOTHING;

-- ============================================
-- 10. Audit Logs
-- ============================================

INSERT INTO audit_logs (id, server_id, user_id, action_type, target_type, target_id, changes, reason, created_at) VALUES
  (1, 1, 1, 'SERVER_CREATE', 'server', '1', '{"name": "Development Server"}', NULL, NOW() - INTERVAL '30 days'),
  (2, 1, 1, 'CHANNEL_CREATE', 'channel', '2', '{"name": "general", "type": "text"}', 'Initial channel setup', NOW() - INTERVAL '30 days'),
  (3, 1, 1, 'ROLE_CREATE', 'role', '2', '{"name": "Admin", "permissions": 8589934591}', NULL, NOW() - INTERVAL '30 days'),
  (4, 1, 1, 'MEMBER_ROLE_UPDATE', 'member', '2', '{"added_roles": [3]}', 'Promoted to moderator', NOW() - INTERVAL '25 days'),
  (5, 2, 2, 'SERVER_CREATE', 'server', '2', '{"name": "Gaming Hub"}', NULL, NOW() - INTERVAL '20 days'),
  (6, 3, 3, 'SERVER_CREATE', 'server', '3', '{"name": "Study Group"}', NULL, NOW() - INTERVAL '10 days')
ON CONFLICT (id) DO NOTHING;

SELECT setval('audit_logs_id_seq', (SELECT MAX(id) FROM audit_logs));

COMMIT;

-- ============================================
-- Verification Queries
-- ============================================

SELECT 'Users: ' || COUNT(*) as count FROM users;
SELECT 'Servers: ' || COUNT(*) as count FROM servers;
SELECT 'Channels: ' || COUNT(*) as count FROM channels;
SELECT 'Roles: ' || COUNT(*) as count FROM roles;
SELECT 'Messages: ' || COUNT(*) as count FROM messages;
SELECT 'Server Members: ' || COUNT(*) as count FROM server_members;

SELECT '✅ Seed data loaded successfully!' as status;
