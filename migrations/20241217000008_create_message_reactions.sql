-- Migration: Create message_reactions table
-- Description: Stores emoji reactions on messages (both Unicode and custom emojis)

-- Message reactions table
-- Composite primary key ensures one reaction per user per emoji per message
CREATE TABLE message_reactions (
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji VARCHAR(100) NOT NULL,  -- Unicode emoji or custom emoji ID (e.g., "thumbsup" or "123456789")
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite primary key: one reaction type per user per message
    PRIMARY KEY (message_id, user_id, emoji),

    -- Ensure emoji is not empty
    CONSTRAINT message_reactions_emoji_not_empty CHECK (char_length(emoji) > 0)
);

-- Index for fetching all reactions on a message (most common query)
CREATE INDEX idx_message_reactions_message ON message_reactions(message_id);

-- Index for counting reactions by emoji type on a message
-- Useful for "5 users reacted with thumbsup" queries
CREATE INDEX idx_message_reactions_emoji ON message_reactions(message_id, emoji);

-- Index for finding all reactions by a specific user
-- Useful for "reactions you've added" features
CREATE INDEX idx_message_reactions_user ON message_reactions(user_id);

COMMENT ON TABLE message_reactions IS 'Emoji reactions on messages';
COMMENT ON COLUMN message_reactions.emoji IS 'Unicode emoji character or custom emoji snowflake ID';
COMMENT ON COLUMN message_reactions.created_at IS 'When the reaction was added';
