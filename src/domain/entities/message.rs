//! Message entity and repository trait.
//!
//! Maps to the `messages` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Message types matching the PostgreSQL ENUM `message_type`.
///
/// Database definition:
/// ```sql
/// CREATE TYPE message_type AS ENUM (
///     'default',           -- Regular user message
///     'recipient_add',     -- User added to group DM
///     'recipient_remove',  -- User removed from group DM
///     'call',              -- Voice/video call started
///     'channel_name_change',
///     'channel_icon_change',
///     'channel_pinned_message',
///     'guild_member_join', -- Welcome message when user joins server
///     'reply'              -- Reply to another message
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// A regular user message
    #[default]
    Default,
    /// A recipient was added to a group DM
    RecipientAdd,
    /// A recipient was removed from a group DM
    RecipientRemove,
    /// A voice/video call was started
    Call,
    /// Channel name was changed
    ChannelNameChange,
    /// Channel icon was changed
    ChannelIconChange,
    /// A message was pinned
    ChannelPinnedMessage,
    /// A new member joined the server
    GuildMemberJoin,
    /// A reply to another message
    Reply,
}

impl MessageType {
    /// Convert from database string representation.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "default" => Self::Default,
            "recipient_add" => Self::RecipientAdd,
            "recipient_remove" => Self::RecipientRemove,
            "call" => Self::Call,
            "channel_name_change" => Self::ChannelNameChange,
            "channel_icon_change" => Self::ChannelIconChange,
            "channel_pinned_message" => Self::ChannelPinnedMessage,
            "guild_member_join" => Self::GuildMemberJoin,
            "reply" => Self::Reply,
            _ => Self::Default,
        }
    }

    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::RecipientAdd => "recipient_add",
            Self::RecipientRemove => "recipient_remove",
            Self::Call => "call",
            Self::ChannelNameChange => "channel_name_change",
            Self::ChannelIconChange => "channel_icon_change",
            Self::ChannelPinnedMessage => "channel_pinned_message",
            Self::GuildMemberJoin => "guild_member_join",
            Self::Reply => "reply",
        }
    }

    /// Check if this is a system message type.
    pub fn is_system(&self) -> bool {
        !matches!(self, Self::Default | Self::Reply)
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a message in a channel.
///
/// Maps to the `messages` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - channel_id: BIGINT NOT NULL REFERENCES channels(id)
/// - author_id: BIGINT NOT NULL REFERENCES users(id)
/// - content: TEXT NOT NULL (max 4000 characters)
/// - message_type: message_type NOT NULL DEFAULT 'default'
/// - reply_to_id: BIGINT REFERENCES messages(id) -- For reply messages
/// - pinned: BOOLEAN NOT NULL DEFAULT FALSE
/// - edited_at: TIMESTAMPTZ NULL
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Channel ID where the message was sent
    pub channel_id: i64,

    /// Author user ID
    pub author_id: i64,

    /// Message content (up to 4000 characters)
    pub content: String,

    /// Type of message
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// ID of the message being replied to (if this is a reply)
    pub reply_to_id: Option<i64>,

    /// Whether message is pinned
    pub pinned: bool,

    /// Timestamp when message was last edited (None if never edited)
    pub edited_at: Option<DateTime<Utc>>,

    /// Timestamp when message was sent
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Check if this message has been edited.
    pub fn is_edited(&self) -> bool {
        self.edited_at.is_some()
    }

    /// Check if this is a reply message.
    pub fn is_reply(&self) -> bool {
        self.reply_to_id.is_some()
    }

    /// Check if this is a system message.
    pub fn is_system(&self) -> bool {
        self.message_type.is_system()
    }

    /// Get the content length in characters.
    pub fn content_length(&self) -> usize {
        self.content.chars().count()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: 0,
            channel_id: 0,
            author_id: 0,
            content: String::new(),
            message_type: MessageType::default(),
            reply_to_id: None,
            pinned: false,
            edited_at: None,
            created_at: Utc::now(),
        }
    }
}

/// Repository trait for Message data access operations.
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// Find a message by its Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Message>, AppError>;

    /// Find messages in a channel with cursor-based pagination.
    ///
    /// Uses keyset pagination for optimal performance on large datasets.
    /// - `before`: Get messages before this message ID (descending)
    /// - `after`: Get messages after this message ID (ascending)
    /// - `limit`: Maximum number of messages to return
    async fn find_by_channel(
        &self,
        channel_id: i64,
        before: Option<i64>,
        after: Option<i64>,
        limit: i32,
    ) -> Result<Vec<Message>, AppError>;

    /// Find pinned messages in a channel.
    async fn find_pinned(&self, channel_id: i64) -> Result<Vec<Message>, AppError>;

    /// Find messages by author in a channel.
    async fn find_by_author(
        &self,
        channel_id: i64,
        author_id: i64,
        limit: i32,
    ) -> Result<Vec<Message>, AppError>;

    /// Create a new message.
    async fn create(&self, message: &Message) -> Result<Message, AppError>;

    /// Update a message (for editing content).
    async fn update(&self, message: &Message) -> Result<Message, AppError>;

    /// Delete a message.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Bulk delete messages (up to 100 at a time).
    async fn bulk_delete(&self, channel_id: i64, message_ids: Vec<i64>) -> Result<(), AppError>;

    /// Pin a message.
    async fn pin(&self, id: i64) -> Result<(), AppError>;

    /// Unpin a message.
    async fn unpin(&self, id: i64) -> Result<(), AppError>;

    /// Get the count of messages in a channel.
    async fn count_by_channel(&self, channel_id: i64) -> Result<i64, AppError>;
}
