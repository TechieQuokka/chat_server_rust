//! Role entity and repository trait.
//!
//! Maps to the `roles` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Represents a role in a server.
///
/// Roles define permissions and can be assigned to members.
///
/// Maps to the `roles` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - server_id: BIGINT NOT NULL REFERENCES servers(id)
/// - name: VARCHAR(100) NOT NULL
/// - permissions: BIGINT NOT NULL DEFAULT 0 (64-bit permission flags)
/// - position: INTEGER NOT NULL DEFAULT 0
/// - color: INTEGER NULL (RGB color value)
/// - hoist: BOOLEAN NOT NULL DEFAULT FALSE
/// - mentionable: BOOLEAN NOT NULL DEFAULT FALSE
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - updated_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Server ID this role belongs to
    pub server_id: i64,

    /// Role name (up to 100 characters)
    pub name: String,

    /// Permission bitfield (64-bit flags)
    pub permissions: i64,

    /// Position in the role hierarchy (higher = more priority)
    pub position: i32,

    /// Role color (RGB integer, None for default/no color)
    pub color: Option<i32>,

    /// Whether this role is hoisted (shown separately in member list)
    pub hoist: bool,

    /// Whether this role is mentionable by everyone
    pub mentionable: bool,

    /// Role creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Permission flags for roles (64-bit bitfield).
#[allow(dead_code)]
pub mod permissions {
    /// Create instant invites
    pub const CREATE_INSTANT_INVITE: i64 = 1 << 0;
    /// Kick members from the server
    pub const KICK_MEMBERS: i64 = 1 << 1;
    /// Ban members from the server
    pub const BAN_MEMBERS: i64 = 1 << 2;
    /// Administrator (full permissions)
    pub const ADMINISTRATOR: i64 = 1 << 3;
    /// Manage channels
    pub const MANAGE_CHANNELS: i64 = 1 << 4;
    /// Manage the server
    pub const MANAGE_GUILD: i64 = 1 << 5;
    /// Add reactions to messages
    pub const ADD_REACTIONS: i64 = 1 << 6;
    /// View audit log
    pub const VIEW_AUDIT_LOG: i64 = 1 << 7;
    /// Priority speaker in voice channels
    pub const PRIORITY_SPEAKER: i64 = 1 << 8;
    /// Stream video in voice channels
    pub const STREAM: i64 = 1 << 9;
    /// View channels
    pub const VIEW_CHANNEL: i64 = 1 << 10;
    /// Send messages
    pub const SEND_MESSAGES: i64 = 1 << 11;
    /// Send TTS messages
    pub const SEND_TTS_MESSAGES: i64 = 1 << 12;
    /// Manage messages
    pub const MANAGE_MESSAGES: i64 = 1 << 13;
    /// Embed links
    pub const EMBED_LINKS: i64 = 1 << 14;
    /// Attach files
    pub const ATTACH_FILES: i64 = 1 << 15;
    /// Read message history
    pub const READ_MESSAGE_HISTORY: i64 = 1 << 16;
    /// Mention @everyone
    pub const MENTION_EVERYONE: i64 = 1 << 17;
    /// Use external emojis
    pub const USE_EXTERNAL_EMOJIS: i64 = 1 << 18;
    /// View server insights
    pub const VIEW_GUILD_INSIGHTS: i64 = 1 << 19;
    /// Connect to voice channels
    pub const CONNECT: i64 = 1 << 20;
    /// Speak in voice channels
    pub const SPEAK: i64 = 1 << 21;
    /// Mute members in voice
    pub const MUTE_MEMBERS: i64 = 1 << 22;
    /// Deafen members in voice
    pub const DEAFEN_MEMBERS: i64 = 1 << 23;
    /// Move members between voice channels
    pub const MOVE_MEMBERS: i64 = 1 << 24;
    /// Use voice activity detection
    pub const USE_VAD: i64 = 1 << 25;
    /// Change own nickname
    pub const CHANGE_NICKNAME: i64 = 1 << 26;
    /// Manage nicknames of other members
    pub const MANAGE_NICKNAMES: i64 = 1 << 27;
    /// Manage roles
    pub const MANAGE_ROLES: i64 = 1 << 28;
    /// Manage webhooks
    pub const MANAGE_WEBHOOKS: i64 = 1 << 29;
    /// Manage emojis and stickers
    pub const MANAGE_EMOJIS_AND_STICKERS: i64 = 1 << 30;
}

impl Role {
    /// Check if this role has a specific permission.
    pub fn has_permission(&self, permission: i64) -> bool {
        // Administrator permission overrides all
        if self.permissions & permissions::ADMINISTRATOR != 0 {
            return true;
        }
        self.permissions & permission == permission
    }

    /// Check if this role has Administrator permission.
    pub fn is_admin(&self) -> bool {
        self.permissions & permissions::ADMINISTRATOR != 0
    }

    /// Get the color as an RGB tuple.
    pub fn color_rgb(&self) -> Option<(u8, u8, u8)> {
        self.color.map(|c| {
            let r = ((c >> 16) & 0xFF) as u8;
            let g = ((c >> 8) & 0xFF) as u8;
            let b = (c & 0xFF) as u8;
            (r, g, b)
        })
    }

    /// Get the color as a hex string (e.g., "#FF5733").
    pub fn color_hex(&self) -> Option<String> {
        self.color.map(|c| format!("#{:06X}", c))
    }
}

impl Default for Role {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            server_id: 0,
            name: "new role".to_string(),
            permissions: 0,
            position: 0,
            color: None,
            hoist: false,
            mentionable: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Repository trait for Role data access operations.
#[async_trait]
pub trait RoleRepository: Send + Sync {
    /// Find a role by its Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError>;

    /// Find all roles in a server, ordered by position.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<Role>, AppError>;

    /// Find the @everyone role for a server.
    /// The @everyone role has the same ID as the server.
    async fn find_everyone_role(&self, server_id: i64) -> Result<Option<Role>, AppError>;

    /// Create a new role.
    async fn create(&self, role: &Role) -> Result<Role, AppError>;

    /// Update an existing role.
    async fn update(&self, role: &Role) -> Result<Role, AppError>;

    /// Delete a role.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Update role positions (for reordering).
    async fn update_positions(&self, server_id: i64, positions: Vec<(i64, i32)>) -> Result<(), AppError>;

    /// Get the highest role position for a server (for new role creation).
    async fn get_max_position(&self, server_id: i64) -> Result<i32, AppError>;
}
