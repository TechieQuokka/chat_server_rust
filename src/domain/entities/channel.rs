//! Channel entity and repository trait.
//!
//! Maps to the `channels` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Channel types matching the PostgreSQL ENUM `channel_type`.
///
/// Database definition:
/// ```sql
/// CREATE TYPE channel_type AS ENUM ('text', 'voice', 'category', 'dm', 'group_dm');
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// A text channel within a server
    #[default]
    Text,
    /// A voice channel within a server
    Voice,
    /// A category that contains channels
    Category,
    /// A direct message between two users
    Dm,
    /// A direct message between multiple users
    GroupDm,
}

impl ChannelType {
    /// Convert from database string representation.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "text" => Self::Text,
            "voice" => Self::Voice,
            "category" => Self::Category,
            "dm" => Self::Dm,
            "group_dm" => Self::GroupDm,
            _ => Self::Text,
        }
    }

    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Voice => "voice",
            Self::Category => "category",
            Self::Dm => "dm",
            Self::GroupDm => "group_dm",
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a channel in the chat system.
///
/// Maps to the `channels` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - server_id: BIGINT REFERENCES servers(id) -- NULL for DMs
/// - name: VARCHAR(100) NOT NULL
/// - type: channel_type NOT NULL DEFAULT 'text'
/// - topic: TEXT NULL
/// - position: INTEGER NOT NULL DEFAULT 0
/// - parent_id: BIGINT REFERENCES channels(id) -- Category reference
/// - nsfw: BOOLEAN NOT NULL DEFAULT FALSE
/// - rate_limit_per_user: INTEGER DEFAULT 0 -- Slowmode in seconds
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - updated_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Server ID (None for DM channels)
    pub server_id: Option<i64>,

    /// Channel name (1-100 characters)
    pub name: String,

    /// Channel type
    #[serde(rename = "type")]
    pub channel_type: ChannelType,

    /// Channel topic/description
    pub topic: Option<String>,

    /// Sorting position within category or server
    pub position: i32,

    /// Parent category ID (for channels within a category)
    pub parent_id: Option<i64>,

    /// Whether the channel is NSFW (age-restricted)
    pub nsfw: bool,

    /// Slowmode rate limit (seconds between messages per user)
    pub rate_limit_per_user: i32,

    /// Channel creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Channel {
    /// Check if this is a text-based channel (can send messages).
    pub fn is_text_based(&self) -> bool {
        matches!(
            self.channel_type,
            ChannelType::Text | ChannelType::Dm | ChannelType::GroupDm
        )
    }

    /// Check if this is a voice-based channel.
    pub fn is_voice_based(&self) -> bool {
        matches!(self.channel_type, ChannelType::Voice)
    }

    /// Check if this is a category channel.
    pub fn is_category(&self) -> bool {
        matches!(self.channel_type, ChannelType::Category)
    }

    /// Check if this is a DM channel (direct or group).
    pub fn is_dm(&self) -> bool {
        matches!(self.channel_type, ChannelType::Dm | ChannelType::GroupDm)
    }

    /// Check if this channel belongs to a server.
    pub fn is_server_channel(&self) -> bool {
        self.server_id.is_some()
    }
}

impl Default for Channel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            server_id: None,
            name: String::new(),
            channel_type: ChannelType::default(),
            topic: None,
            position: 0,
            parent_id: None,
            nsfw: false,
            rate_limit_per_user: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Permission overwrite for a channel.
///
/// Maps to the `channel_overwrites` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    /// Channel ID this overwrite belongs to
    pub channel_id: i64,

    /// Target ID (role or user ID)
    pub target_id: i64,

    /// Target type: "role" or "member"
    pub target_type: String,

    /// Allowed permissions bitfield
    pub allow: i64,

    /// Denied permissions bitfield
    pub deny: i64,
}

/// Repository trait for Channel data access operations.
#[async_trait]
pub trait ChannelRepository: Send + Sync {
    /// Find a channel by its Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Channel>, AppError>;

    /// Find all channels in a server.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<Channel>, AppError>;

    /// Find channels by parent category ID.
    async fn find_by_parent_id(&self, parent_id: i64) -> Result<Vec<Channel>, AppError>;

    /// Find DM channel between two users.
    async fn find_dm_channel(&self, user1_id: i64, user2_id: i64) -> Result<Option<Channel>, AppError>;

    /// Create a new channel.
    async fn create(&self, channel: &Channel) -> Result<Channel, AppError>;

    /// Update an existing channel.
    async fn update(&self, channel: &Channel) -> Result<Channel, AppError>;

    /// Delete a channel.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Update channel positions (for reordering).
    async fn update_positions(&self, server_id: i64, positions: Vec<(i64, i32)>) -> Result<(), AppError>;

    /// Get permission overwrites for a channel.
    async fn get_permission_overwrites(&self, channel_id: i64) -> Result<Vec<PermissionOverwrite>, AppError>;

    /// Set permission overwrites for a channel.
    async fn set_permission_overwrites(
        &self,
        channel_id: i64,
        overwrites: Vec<PermissionOverwrite>,
    ) -> Result<(), AppError>;
}
