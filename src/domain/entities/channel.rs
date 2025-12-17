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

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // ChannelType Tests
    // ==========================================================================

    #[test]
    fn test_channel_type_default_is_text() {
        let channel_type = ChannelType::default();
        assert_eq!(channel_type, ChannelType::Text);
    }

    #[test]
    fn test_channel_type_from_str_text() {
        assert_eq!(ChannelType::from_str("text"), ChannelType::Text);
        assert_eq!(ChannelType::from_str("TEXT"), ChannelType::Text);
        assert_eq!(ChannelType::from_str("Text"), ChannelType::Text);
    }

    #[test]
    fn test_channel_type_from_str_voice() {
        assert_eq!(ChannelType::from_str("voice"), ChannelType::Voice);
        assert_eq!(ChannelType::from_str("VOICE"), ChannelType::Voice);
    }

    #[test]
    fn test_channel_type_from_str_category() {
        assert_eq!(ChannelType::from_str("category"), ChannelType::Category);
        assert_eq!(ChannelType::from_str("CATEGORY"), ChannelType::Category);
    }

    #[test]
    fn test_channel_type_from_str_dm() {
        assert_eq!(ChannelType::from_str("dm"), ChannelType::Dm);
        assert_eq!(ChannelType::from_str("DM"), ChannelType::Dm);
    }

    #[test]
    fn test_channel_type_from_str_group_dm() {
        assert_eq!(ChannelType::from_str("group_dm"), ChannelType::GroupDm);
        assert_eq!(ChannelType::from_str("GROUP_DM"), ChannelType::GroupDm);
    }

    #[test]
    fn test_channel_type_from_str_unknown_defaults_to_text() {
        assert_eq!(ChannelType::from_str("unknown"), ChannelType::Text);
        assert_eq!(ChannelType::from_str(""), ChannelType::Text);
        assert_eq!(ChannelType::from_str("invalid"), ChannelType::Text);
    }

    #[test]
    fn test_channel_type_as_str_roundtrip() {
        let types = vec![
            ChannelType::Text,
            ChannelType::Voice,
            ChannelType::Category,
            ChannelType::Dm,
            ChannelType::GroupDm,
        ];

        for channel_type in types {
            let s = channel_type.as_str();
            let parsed = ChannelType::from_str(s);
            assert_eq!(parsed, channel_type, "Roundtrip failed for {:?}", channel_type);
        }
    }

    #[test]
    fn test_channel_type_as_str_values() {
        assert_eq!(ChannelType::Text.as_str(), "text");
        assert_eq!(ChannelType::Voice.as_str(), "voice");
        assert_eq!(ChannelType::Category.as_str(), "category");
        assert_eq!(ChannelType::Dm.as_str(), "dm");
        assert_eq!(ChannelType::GroupDm.as_str(), "group_dm");
    }

    #[test]
    fn test_channel_type_display() {
        assert_eq!(format!("{}", ChannelType::Text), "text");
        assert_eq!(format!("{}", ChannelType::Voice), "voice");
        assert_eq!(format!("{}", ChannelType::Category), "category");
        assert_eq!(format!("{}", ChannelType::Dm), "dm");
        assert_eq!(format!("{}", ChannelType::GroupDm), "group_dm");
    }

    // ==========================================================================
    // Channel Entity Tests
    // ==========================================================================

    fn create_test_channel(channel_type: ChannelType, server_id: Option<i64>) -> Channel {
        Channel {
            id: 12345678901234567,
            server_id,
            name: "test-channel".to_string(),
            channel_type,
            topic: None,
            position: 0,
            parent_id: None,
            nsfw: false,
            rate_limit_per_user: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_channel_default() {
        let channel = Channel::default();

        assert_eq!(channel.id, 0);
        assert!(channel.server_id.is_none());
        assert!(channel.name.is_empty());
        assert_eq!(channel.channel_type, ChannelType::Text);
        assert!(channel.topic.is_none());
        assert_eq!(channel.position, 0);
        assert!(channel.parent_id.is_none());
        assert!(!channel.nsfw);
        assert_eq!(channel.rate_limit_per_user, 0);
    }

    // ==========================================================================
    // is_text_based Tests
    // ==========================================================================

    #[test]
    fn test_channel_is_text_based_true_for_text() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(channel.is_text_based());
    }

    #[test]
    fn test_channel_is_text_based_true_for_dm() {
        let channel = create_test_channel(ChannelType::Dm, None);
        assert!(channel.is_text_based());
    }

    #[test]
    fn test_channel_is_text_based_true_for_group_dm() {
        let channel = create_test_channel(ChannelType::GroupDm, None);
        assert!(channel.is_text_based());
    }

    #[test]
    fn test_channel_is_text_based_false_for_voice() {
        let channel = create_test_channel(ChannelType::Voice, Some(100));
        assert!(!channel.is_text_based());
    }

    #[test]
    fn test_channel_is_text_based_false_for_category() {
        let channel = create_test_channel(ChannelType::Category, Some(100));
        assert!(!channel.is_text_based());
    }

    // ==========================================================================
    // is_voice_based Tests
    // ==========================================================================

    #[test]
    fn test_channel_is_voice_based_true_for_voice() {
        let channel = create_test_channel(ChannelType::Voice, Some(100));
        assert!(channel.is_voice_based());
    }

    #[test]
    fn test_channel_is_voice_based_false_for_text() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(!channel.is_voice_based());
    }

    #[test]
    fn test_channel_is_voice_based_false_for_dm() {
        let channel = create_test_channel(ChannelType::Dm, None);
        assert!(!channel.is_voice_based());
    }

    #[test]
    fn test_channel_is_voice_based_false_for_category() {
        let channel = create_test_channel(ChannelType::Category, Some(100));
        assert!(!channel.is_voice_based());
    }

    // ==========================================================================
    // is_category Tests
    // ==========================================================================

    #[test]
    fn test_channel_is_category_true_for_category() {
        let channel = create_test_channel(ChannelType::Category, Some(100));
        assert!(channel.is_category());
    }

    #[test]
    fn test_channel_is_category_false_for_text() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(!channel.is_category());
    }

    #[test]
    fn test_channel_is_category_false_for_voice() {
        let channel = create_test_channel(ChannelType::Voice, Some(100));
        assert!(!channel.is_category());
    }

    // ==========================================================================
    // is_dm Tests
    // ==========================================================================

    #[test]
    fn test_channel_is_dm_true_for_dm() {
        let channel = create_test_channel(ChannelType::Dm, None);
        assert!(channel.is_dm());
    }

    #[test]
    fn test_channel_is_dm_true_for_group_dm() {
        let channel = create_test_channel(ChannelType::GroupDm, None);
        assert!(channel.is_dm());
    }

    #[test]
    fn test_channel_is_dm_false_for_text() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(!channel.is_dm());
    }

    #[test]
    fn test_channel_is_dm_false_for_voice() {
        let channel = create_test_channel(ChannelType::Voice, Some(100));
        assert!(!channel.is_dm());
    }

    #[test]
    fn test_channel_is_dm_false_for_category() {
        let channel = create_test_channel(ChannelType::Category, Some(100));
        assert!(!channel.is_dm());
    }

    // ==========================================================================
    // is_server_channel Tests
    // ==========================================================================

    #[test]
    fn test_channel_is_server_channel_true_when_has_server_id() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(channel.is_server_channel());
    }

    #[test]
    fn test_channel_is_server_channel_false_when_no_server_id() {
        let channel = create_test_channel(ChannelType::Dm, None);
        assert!(!channel.is_server_channel());
    }

    // ==========================================================================
    // Channel with Parent Category Tests
    // ==========================================================================

    #[test]
    fn test_channel_with_parent_id() {
        let mut channel = create_test_channel(ChannelType::Text, Some(100));
        channel.parent_id = Some(200);

        assert_eq!(channel.parent_id, Some(200));
    }

    #[test]
    fn test_channel_without_parent_id() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(channel.parent_id.is_none());
    }

    // ==========================================================================
    // Channel NSFW Tests
    // ==========================================================================

    #[test]
    fn test_channel_nsfw_default_false() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert!(!channel.nsfw);
    }

    #[test]
    fn test_channel_nsfw_can_be_set() {
        let mut channel = create_test_channel(ChannelType::Text, Some(100));
        channel.nsfw = true;
        assert!(channel.nsfw);
    }

    // ==========================================================================
    // Channel Rate Limit Tests
    // ==========================================================================

    #[test]
    fn test_channel_rate_limit_default_zero() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert_eq!(channel.rate_limit_per_user, 0);
    }

    #[test]
    fn test_channel_rate_limit_can_be_set() {
        let mut channel = create_test_channel(ChannelType::Text, Some(100));
        channel.rate_limit_per_user = 5; // 5 second slowmode
        assert_eq!(channel.rate_limit_per_user, 5);
    }

    // ==========================================================================
    // Channel Serialization Tests
    // ==========================================================================

    #[test]
    fn test_channel_type_serializes_as_type() {
        let channel = create_test_channel(ChannelType::Text, Some(100));

        let serialized = serde_json::to_string(&channel).expect("Failed to serialize channel");

        // channel_type should be serialized as "type" due to #[serde(rename = "type")]
        assert!(serialized.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_channel_serialization_includes_all_fields() {
        let mut channel = create_test_channel(ChannelType::Voice, Some(100));
        channel.topic = Some("Test topic".to_string());
        channel.nsfw = true;
        channel.rate_limit_per_user = 10;

        let serialized = serde_json::to_string(&channel).expect("Failed to serialize channel");

        assert!(serialized.contains("\"id\":12345678901234567"));
        assert!(serialized.contains("\"server_id\":100"));
        assert!(serialized.contains("\"name\":\"test-channel\""));
        assert!(serialized.contains("\"type\":\"voice\""));
        assert!(serialized.contains("\"topic\":\"Test topic\""));
        assert!(serialized.contains("\"nsfw\":true"));
        assert!(serialized.contains("\"rate_limit_per_user\":10"));
    }

    // ==========================================================================
    // PermissionOverwrite Tests
    // ==========================================================================

    #[test]
    fn test_permission_overwrite_creation() {
        let overwrite = PermissionOverwrite {
            channel_id: 100,
            target_id: 200,
            target_type: "role".to_string(),
            allow: 1024, // VIEW_CHANNEL
            deny: 2048,  // SEND_MESSAGES
        };

        assert_eq!(overwrite.channel_id, 100);
        assert_eq!(overwrite.target_id, 200);
        assert_eq!(overwrite.target_type, "role");
        assert_eq!(overwrite.allow, 1024);
        assert_eq!(overwrite.deny, 2048);
    }

    #[test]
    fn test_permission_overwrite_for_member() {
        let overwrite = PermissionOverwrite {
            channel_id: 100,
            target_id: 300, // user_id
            target_type: "member".to_string(),
            allow: 0,
            deny: 0,
        };

        assert_eq!(overwrite.target_type, "member");
    }

    #[test]
    fn test_permission_overwrite_clone() {
        let overwrite = PermissionOverwrite {
            channel_id: 100,
            target_id: 200,
            target_type: "role".to_string(),
            allow: 1024,
            deny: 2048,
        };

        let cloned = overwrite.clone();

        assert_eq!(overwrite.channel_id, cloned.channel_id);
        assert_eq!(overwrite.target_id, cloned.target_id);
        assert_eq!(overwrite.target_type, cloned.target_type);
        assert_eq!(overwrite.allow, cloned.allow);
        assert_eq!(overwrite.deny, cloned.deny);
    }

    // ==========================================================================
    // Channel Clone Tests
    // ==========================================================================

    #[test]
    fn test_channel_clone() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        let cloned = channel.clone();

        assert_eq!(channel.id, cloned.id);
        assert_eq!(channel.server_id, cloned.server_id);
        assert_eq!(channel.name, cloned.name);
        assert_eq!(channel.channel_type, cloned.channel_type);
    }

    // ==========================================================================
    // ChannelType Copy Tests
    // ==========================================================================

    #[test]
    fn test_channel_type_is_copy() {
        let ct1 = ChannelType::Text;
        let ct2 = ct1; // Copy

        assert_eq!(ct1, ct2);
    }

    // ==========================================================================
    // Channel Position Tests
    // ==========================================================================

    #[test]
    fn test_channel_position_default_zero() {
        let channel = create_test_channel(ChannelType::Text, Some(100));
        assert_eq!(channel.position, 0);
    }

    #[test]
    fn test_channel_position_can_be_set() {
        let mut channel = create_test_channel(ChannelType::Text, Some(100));
        channel.position = 5;
        assert_eq!(channel.position, 5);
    }

    #[test]
    fn test_channel_position_ordering() {
        let mut channels = vec![
            {
                let mut c = create_test_channel(ChannelType::Text, Some(100));
                c.position = 2;
                c.name = "channel-c".to_string();
                c
            },
            {
                let mut c = create_test_channel(ChannelType::Text, Some(100));
                c.position = 0;
                c.name = "channel-a".to_string();
                c
            },
            {
                let mut c = create_test_channel(ChannelType::Text, Some(100));
                c.position = 1;
                c.name = "channel-b".to_string();
                c
            },
        ];

        channels.sort_by_key(|c| c.position);

        assert_eq!(channels[0].name, "channel-a");
        assert_eq!(channels[1].name, "channel-b");
        assert_eq!(channels[2].name, "channel-c");
    }
}
