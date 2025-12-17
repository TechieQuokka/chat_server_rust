//! Message Reaction entity and repository trait.
//!
//! Maps to the `message_reactions` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Represents a reaction on a message.
///
/// Maps to the `message_reactions` table:
/// - message_id: BIGINT NOT NULL REFERENCES messages(id) (composite PK)
/// - user_id: BIGINT NOT NULL REFERENCES users(id) (composite PK)
/// - emoji: VARCHAR(100) NOT NULL (composite PK)
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
///
/// The composite primary key (message_id, user_id, emoji) ensures
/// one reaction per user per emoji per message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    /// Message ID this reaction is on
    pub message_id: i64,

    /// User ID who added the reaction
    pub user_id: i64,

    /// Emoji identifier (Unicode emoji or custom emoji snowflake ID)
    pub emoji: String,

    /// When the reaction was added
    pub created_at: DateTime<Utc>,
}

impl Reaction {
    /// Create a new reaction.
    pub fn new(message_id: i64, user_id: i64, emoji: String) -> Self {
        Self {
            message_id,
            user_id,
            emoji,
            created_at: Utc::now(),
        }
    }

    /// Check if this is a custom emoji (snowflake ID) vs Unicode emoji.
    pub fn is_custom_emoji(&self) -> bool {
        // Custom emojis are stored as numeric snowflake IDs
        self.emoji.chars().all(|c| c.is_ascii_digit())
    }
}

impl Default for Reaction {
    fn default() -> Self {
        Self {
            message_id: 0,
            user_id: 0,
            emoji: String::new(),
            created_at: Utc::now(),
        }
    }
}

/// Aggregated reaction count for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionCount {
    /// Emoji identifier
    pub emoji: String,

    /// Number of users who reacted with this emoji
    pub count: i64,

    /// Whether the current user has reacted with this emoji
    #[serde(default)]
    pub me: bool,
}

impl ReactionCount {
    /// Create a new reaction count.
    pub fn new(emoji: String, count: i64, me: bool) -> Self {
        Self { emoji, count, me }
    }
}

/// Repository trait for Reaction data access operations.
#[async_trait]
pub trait ReactionRepository: Send + Sync {
    /// Add a reaction to a message.
    async fn add(&self, reaction: &Reaction) -> Result<(), AppError>;

    /// Remove a reaction from a message.
    async fn remove(&self, message_id: i64, user_id: i64, emoji: &str) -> Result<(), AppError>;

    /// Remove all reactions of a specific emoji from a message.
    async fn remove_emoji(&self, message_id: i64, emoji: &str) -> Result<(), AppError>;

    /// Remove all reactions from a message.
    async fn remove_all(&self, message_id: i64) -> Result<(), AppError>;

    /// Get all users who reacted with a specific emoji on a message.
    async fn get_users(
        &self,
        message_id: i64,
        emoji: &str,
        after: Option<i64>,
        limit: i32,
    ) -> Result<Vec<i64>, AppError>;

    /// Get aggregated reaction counts for a message.
    async fn get_counts(&self, message_id: i64) -> Result<Vec<ReactionCount>, AppError>;

    /// Get aggregated reaction counts with "me" flag for the current user.
    async fn get_counts_for_user(
        &self,
        message_id: i64,
        user_id: i64,
    ) -> Result<Vec<ReactionCount>, AppError>;

    /// Check if a user has reacted with a specific emoji.
    async fn has_reacted(&self, message_id: i64, user_id: i64, emoji: &str) -> Result<bool, AppError>;

    /// Get all reactions by a user on a message.
    async fn get_user_reactions(&self, message_id: i64, user_id: i64) -> Result<Vec<String>, AppError>;
}
