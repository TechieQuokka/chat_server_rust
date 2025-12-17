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

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // MessageType Tests
    // ==========================================================================

    #[test]
    fn test_message_type_default_is_default() {
        let msg_type = MessageType::default();
        assert_eq!(msg_type, MessageType::Default);
    }

    #[test]
    fn test_message_type_from_str_default() {
        assert_eq!(MessageType::from_str("default"), MessageType::Default);
        assert_eq!(MessageType::from_str("DEFAULT"), MessageType::Default);
    }

    #[test]
    fn test_message_type_from_str_recipient_add() {
        assert_eq!(MessageType::from_str("recipient_add"), MessageType::RecipientAdd);
        assert_eq!(MessageType::from_str("RECIPIENT_ADD"), MessageType::RecipientAdd);
    }

    #[test]
    fn test_message_type_from_str_recipient_remove() {
        assert_eq!(MessageType::from_str("recipient_remove"), MessageType::RecipientRemove);
    }

    #[test]
    fn test_message_type_from_str_call() {
        assert_eq!(MessageType::from_str("call"), MessageType::Call);
    }

    #[test]
    fn test_message_type_from_str_channel_name_change() {
        assert_eq!(MessageType::from_str("channel_name_change"), MessageType::ChannelNameChange);
    }

    #[test]
    fn test_message_type_from_str_channel_icon_change() {
        assert_eq!(MessageType::from_str("channel_icon_change"), MessageType::ChannelIconChange);
    }

    #[test]
    fn test_message_type_from_str_channel_pinned_message() {
        assert_eq!(MessageType::from_str("channel_pinned_message"), MessageType::ChannelPinnedMessage);
    }

    #[test]
    fn test_message_type_from_str_guild_member_join() {
        assert_eq!(MessageType::from_str("guild_member_join"), MessageType::GuildMemberJoin);
    }

    #[test]
    fn test_message_type_from_str_reply() {
        assert_eq!(MessageType::from_str("reply"), MessageType::Reply);
    }

    #[test]
    fn test_message_type_from_str_unknown_defaults_to_default() {
        assert_eq!(MessageType::from_str("unknown"), MessageType::Default);
        assert_eq!(MessageType::from_str(""), MessageType::Default);
        assert_eq!(MessageType::from_str("invalid"), MessageType::Default);
    }

    #[test]
    fn test_message_type_as_str_roundtrip() {
        let types = vec![
            MessageType::Default,
            MessageType::RecipientAdd,
            MessageType::RecipientRemove,
            MessageType::Call,
            MessageType::ChannelNameChange,
            MessageType::ChannelIconChange,
            MessageType::ChannelPinnedMessage,
            MessageType::GuildMemberJoin,
            MessageType::Reply,
        ];

        for msg_type in types {
            let s = msg_type.as_str();
            let parsed = MessageType::from_str(s);
            assert_eq!(parsed, msg_type, "Roundtrip failed for {:?}", msg_type);
        }
    }

    #[test]
    fn test_message_type_as_str_values() {
        assert_eq!(MessageType::Default.as_str(), "default");
        assert_eq!(MessageType::RecipientAdd.as_str(), "recipient_add");
        assert_eq!(MessageType::RecipientRemove.as_str(), "recipient_remove");
        assert_eq!(MessageType::Call.as_str(), "call");
        assert_eq!(MessageType::ChannelNameChange.as_str(), "channel_name_change");
        assert_eq!(MessageType::ChannelIconChange.as_str(), "channel_icon_change");
        assert_eq!(MessageType::ChannelPinnedMessage.as_str(), "channel_pinned_message");
        assert_eq!(MessageType::GuildMemberJoin.as_str(), "guild_member_join");
        assert_eq!(MessageType::Reply.as_str(), "reply");
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(format!("{}", MessageType::Default), "default");
        assert_eq!(format!("{}", MessageType::Reply), "reply");
        assert_eq!(format!("{}", MessageType::GuildMemberJoin), "guild_member_join");
    }

    // ==========================================================================
    // MessageType is_system Tests
    // ==========================================================================

    #[test]
    fn test_message_type_is_system_false_for_default() {
        assert!(!MessageType::Default.is_system());
    }

    #[test]
    fn test_message_type_is_system_false_for_reply() {
        assert!(!MessageType::Reply.is_system());
    }

    #[test]
    fn test_message_type_is_system_true_for_system_types() {
        assert!(MessageType::RecipientAdd.is_system());
        assert!(MessageType::RecipientRemove.is_system());
        assert!(MessageType::Call.is_system());
        assert!(MessageType::ChannelNameChange.is_system());
        assert!(MessageType::ChannelIconChange.is_system());
        assert!(MessageType::ChannelPinnedMessage.is_system());
        assert!(MessageType::GuildMemberJoin.is_system());
    }

    // ==========================================================================
    // Message Entity Tests
    // ==========================================================================

    fn create_test_message() -> Message {
        Message {
            id: 12345678901234567,
            channel_id: 100,
            author_id: 200,
            content: "Hello, world!".to_string(),
            message_type: MessageType::Default,
            reply_to_id: None,
            pinned: false,
            edited_at: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_message_default() {
        let message = Message::default();

        assert_eq!(message.id, 0);
        assert_eq!(message.channel_id, 0);
        assert_eq!(message.author_id, 0);
        assert!(message.content.is_empty());
        assert_eq!(message.message_type, MessageType::Default);
        assert!(message.reply_to_id.is_none());
        assert!(!message.pinned);
        assert!(message.edited_at.is_none());
    }

    // ==========================================================================
    // Message is_edited Tests
    // ==========================================================================

    #[test]
    fn test_message_is_edited_false_when_no_edited_at() {
        let message = create_test_message();
        assert!(!message.is_edited());
    }

    #[test]
    fn test_message_is_edited_true_when_has_edited_at() {
        let mut message = create_test_message();
        message.edited_at = Some(Utc::now());
        assert!(message.is_edited());
    }

    // ==========================================================================
    // Message is_reply Tests
    // ==========================================================================

    #[test]
    fn test_message_is_reply_false_when_no_reply_to_id() {
        let message = create_test_message();
        assert!(!message.is_reply());
    }

    #[test]
    fn test_message_is_reply_true_when_has_reply_to_id() {
        let mut message = create_test_message();
        message.reply_to_id = Some(300);
        assert!(message.is_reply());
    }

    #[test]
    fn test_message_is_reply_with_reply_type() {
        let mut message = create_test_message();
        message.message_type = MessageType::Reply;
        message.reply_to_id = Some(300);

        assert!(message.is_reply());
    }

    // ==========================================================================
    // Message is_system Tests
    // ==========================================================================

    #[test]
    fn test_message_is_system_false_for_default_type() {
        let message = create_test_message();
        assert!(!message.is_system());
    }

    #[test]
    fn test_message_is_system_false_for_reply_type() {
        let mut message = create_test_message();
        message.message_type = MessageType::Reply;
        assert!(!message.is_system());
    }

    #[test]
    fn test_message_is_system_true_for_system_types() {
        let mut message = create_test_message();

        message.message_type = MessageType::GuildMemberJoin;
        assert!(message.is_system());

        message.message_type = MessageType::ChannelPinnedMessage;
        assert!(message.is_system());

        message.message_type = MessageType::Call;
        assert!(message.is_system());
    }

    // ==========================================================================
    // Message content_length Tests
    // ==========================================================================

    #[test]
    fn test_message_content_length_empty() {
        let mut message = create_test_message();
        message.content = String::new();
        assert_eq!(message.content_length(), 0);
    }

    #[test]
    fn test_message_content_length_ascii() {
        let mut message = create_test_message();
        message.content = "Hello".to_string();
        assert_eq!(message.content_length(), 5);
    }

    #[test]
    fn test_message_content_length_unicode() {
        let mut message = create_test_message();
        // Each emoji is a single character despite being multiple bytes
        message.content = "Hello ".to_string();
        // "Hello " = 6 chars, and each emoji counts based on actual char count
        assert!(message.content_length() > 0);
    }

    #[test]
    fn test_message_content_length_multibyte_unicode() {
        let mut message = create_test_message();
        message.content = "Korean".to_string(); // 6 ASCII chars
        assert_eq!(message.content_length(), 6);
    }

    #[test]
    fn test_message_content_length_korean_chars() {
        // Test with actual Korean characters
        let korean_message = Message {
            content: "aB".to_string(), // 2 characters (each Korean character is 1 char in Rust)
            ..create_test_message()
        };
        // "aB" contains 2 characters
        assert_eq!(korean_message.content_length(), 2);

        // Test with proper Korean string
        let korean_message2 = Message {
            content: "\u{D55C}\u{AD6D}\u{C5B4}".to_string(), // Unicode for korean characters
            ..create_test_message()
        };
        assert_eq!(korean_message2.content_length(), 3); // 3 Korean characters
    }

    // ==========================================================================
    // Message Serialization Tests
    // ==========================================================================

    #[test]
    fn test_message_type_serializes_as_type() {
        let message = create_test_message();

        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        // message_type should be serialized as "type" due to #[serde(rename = "type")]
        assert!(serialized.contains("\"type\":\"default\""));
    }

    #[test]
    fn test_message_serialization_includes_required_fields() {
        let message = create_test_message();

        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        assert!(serialized.contains("\"id\":12345678901234567"));
        assert!(serialized.contains("\"channel_id\":100"));
        assert!(serialized.contains("\"author_id\":200"));
        assert!(serialized.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_message_serialization_with_optional_fields() {
        let mut message = create_test_message();
        message.reply_to_id = Some(300);
        message.pinned = true;

        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        assert!(serialized.contains("\"reply_to_id\":300"));
        assert!(serialized.contains("\"pinned\":true"));
    }

    // ==========================================================================
    // Message Clone Tests
    // ==========================================================================

    #[test]
    fn test_message_clone() {
        let message = create_test_message();
        let cloned = message.clone();

        assert_eq!(message.id, cloned.id);
        assert_eq!(message.channel_id, cloned.channel_id);
        assert_eq!(message.author_id, cloned.author_id);
        assert_eq!(message.content, cloned.content);
        assert_eq!(message.message_type, cloned.message_type);
    }

    // ==========================================================================
    // MessageType Copy Tests
    // ==========================================================================

    #[test]
    fn test_message_type_is_copy() {
        let mt1 = MessageType::Default;
        let mt2 = mt1; // Copy

        assert_eq!(mt1, mt2);
    }

    // ==========================================================================
    // Message Pinned Tests
    // ==========================================================================

    #[test]
    fn test_message_pinned_default_false() {
        let message = create_test_message();
        assert!(!message.pinned);
    }

    #[test]
    fn test_message_pinned_can_be_set() {
        let mut message = create_test_message();
        message.pinned = true;
        assert!(message.pinned);
    }

    // ==========================================================================
    // Message Edit Tracking Tests
    // ==========================================================================

    #[test]
    fn test_message_edit_tracking() {
        let mut message = create_test_message();

        // Initially not edited
        assert!(!message.is_edited());
        assert!(message.edited_at.is_none());

        // After edit
        let edit_time = Utc::now();
        message.edited_at = Some(edit_time);

        assert!(message.is_edited());
        assert_eq!(message.edited_at, Some(edit_time));
    }

    // ==========================================================================
    // Message Content Edge Cases
    // ==========================================================================

    #[test]
    fn test_message_content_whitespace_only() {
        let mut message = create_test_message();
        message.content = "   ".to_string();

        assert_eq!(message.content_length(), 3);
    }

    #[test]
    fn test_message_content_newlines() {
        let mut message = create_test_message();
        message.content = "Line 1\nLine 2\nLine 3".to_string();

        // Count includes newline characters
        assert_eq!(message.content_length(), 20);
    }

    #[test]
    fn test_message_content_max_length_boundary() {
        // Discord allows up to 4000 characters
        let mut message = create_test_message();
        message.content = "a".repeat(4000);

        assert_eq!(message.content_length(), 4000);
    }
}
