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

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Role Entity Tests
    // ==========================================================================

    fn create_test_role() -> Role {
        Role {
            id: 12345678901234567,
            server_id: 100,
            name: "Test Role".to_string(),
            permissions: 0,
            position: 1,
            color: None,
            hoist: false,
            mentionable: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_role_default() {
        let role = Role::default();

        assert_eq!(role.id, 0);
        assert_eq!(role.server_id, 0);
        assert_eq!(role.name, "new role");
        assert_eq!(role.permissions, 0);
        assert_eq!(role.position, 0);
        assert!(role.color.is_none());
        assert!(!role.hoist);
        assert!(!role.mentionable);
    }

    // ==========================================================================
    // has_permission Tests
    // ==========================================================================

    #[test]
    fn test_role_has_permission_true_when_set() {
        let mut role = create_test_role();
        role.permissions = permissions::VIEW_CHANNEL;

        assert!(role.has_permission(permissions::VIEW_CHANNEL));
    }

    #[test]
    fn test_role_has_permission_false_when_not_set() {
        let role = create_test_role();

        assert!(!role.has_permission(permissions::VIEW_CHANNEL));
        assert!(!role.has_permission(permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_role_has_permission_multiple() {
        let mut role = create_test_role();
        role.permissions = permissions::VIEW_CHANNEL | permissions::SEND_MESSAGES;

        assert!(role.has_permission(permissions::VIEW_CHANNEL));
        assert!(role.has_permission(permissions::SEND_MESSAGES));
        assert!(!role.has_permission(permissions::MANAGE_MESSAGES));
    }

    #[test]
    fn test_role_has_permission_combined_check() {
        let mut role = create_test_role();
        role.permissions = permissions::VIEW_CHANNEL | permissions::SEND_MESSAGES;

        let required = permissions::VIEW_CHANNEL | permissions::SEND_MESSAGES;
        assert!(role.has_permission(required));

        let too_many = permissions::VIEW_CHANNEL | permissions::ADMINISTRATOR;
        assert!(!role.has_permission(too_many));
    }

    // ==========================================================================
    // Administrator Override Tests
    // ==========================================================================

    #[test]
    fn test_role_admin_has_all_permissions() {
        let mut role = create_test_role();
        role.permissions = permissions::ADMINISTRATOR;

        // Admin should have all permissions via has_permission
        assert!(role.has_permission(permissions::VIEW_CHANNEL));
        assert!(role.has_permission(permissions::SEND_MESSAGES));
        assert!(role.has_permission(permissions::MANAGE_GUILD));
        assert!(role.has_permission(permissions::BAN_MEMBERS));
        assert!(role.has_permission(permissions::MANAGE_ROLES));
    }

    #[test]
    fn test_role_is_admin_true() {
        let mut role = create_test_role();
        role.permissions = permissions::ADMINISTRATOR;

        assert!(role.is_admin());
    }

    #[test]
    fn test_role_is_admin_false() {
        let mut role = create_test_role();
        role.permissions = permissions::MANAGE_GUILD | permissions::BAN_MEMBERS;

        assert!(!role.is_admin());
    }

    // ==========================================================================
    // Color Tests
    // ==========================================================================

    #[test]
    fn test_role_color_rgb_none_when_no_color() {
        let role = create_test_role();
        assert!(role.color_rgb().is_none());
    }

    #[test]
    fn test_role_color_rgb_extracts_components() {
        let mut role = create_test_role();
        // 0xFF5733 = RGB(255, 87, 51)
        role.color = Some(0xFF5733);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 255);
        assert_eq!(g, 87);
        assert_eq!(b, 51);
    }

    #[test]
    fn test_role_color_rgb_pure_red() {
        let mut role = create_test_role();
        role.color = Some(0xFF0000);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_role_color_rgb_pure_green() {
        let mut role = create_test_role();
        role.color = Some(0x00FF00);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_role_color_rgb_pure_blue() {
        let mut role = create_test_role();
        role.color = Some(0x0000FF);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);
    }

    #[test]
    fn test_role_color_rgb_black() {
        let mut role = create_test_role();
        role.color = Some(0x000000);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_role_color_rgb_white() {
        let mut role = create_test_role();
        role.color = Some(0xFFFFFF);

        let (r, g, b) = role.color_rgb().expect("Expected color");
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    #[test]
    fn test_role_color_hex_none_when_no_color() {
        let role = create_test_role();
        assert!(role.color_hex().is_none());
    }

    #[test]
    fn test_role_color_hex_format() {
        let mut role = create_test_role();
        role.color = Some(0xFF5733);

        let hex = role.color_hex().expect("Expected color");
        assert_eq!(hex, "#FF5733");
    }

    #[test]
    fn test_role_color_hex_with_leading_zeros() {
        let mut role = create_test_role();
        role.color = Some(0x00FF00);

        let hex = role.color_hex().expect("Expected color");
        assert_eq!(hex, "#00FF00");
    }

    #[test]
    fn test_role_color_hex_black() {
        let mut role = create_test_role();
        role.color = Some(0x000000);

        let hex = role.color_hex().expect("Expected color");
        assert_eq!(hex, "#000000");
    }

    // ==========================================================================
    // Permission Constants Tests
    // ==========================================================================

    #[test]
    fn test_permission_constant_values() {
        assert_eq!(permissions::CREATE_INSTANT_INVITE, 1 << 0);
        assert_eq!(permissions::KICK_MEMBERS, 1 << 1);
        assert_eq!(permissions::BAN_MEMBERS, 1 << 2);
        assert_eq!(permissions::ADMINISTRATOR, 1 << 3);
        assert_eq!(permissions::MANAGE_CHANNELS, 1 << 4);
        assert_eq!(permissions::MANAGE_GUILD, 1 << 5);
        assert_eq!(permissions::ADD_REACTIONS, 1 << 6);
        assert_eq!(permissions::VIEW_AUDIT_LOG, 1 << 7);
        assert_eq!(permissions::PRIORITY_SPEAKER, 1 << 8);
        assert_eq!(permissions::STREAM, 1 << 9);
        assert_eq!(permissions::VIEW_CHANNEL, 1 << 10);
        assert_eq!(permissions::SEND_MESSAGES, 1 << 11);
        assert_eq!(permissions::SEND_TTS_MESSAGES, 1 << 12);
        assert_eq!(permissions::MANAGE_MESSAGES, 1 << 13);
        assert_eq!(permissions::EMBED_LINKS, 1 << 14);
        assert_eq!(permissions::ATTACH_FILES, 1 << 15);
        assert_eq!(permissions::READ_MESSAGE_HISTORY, 1 << 16);
        assert_eq!(permissions::MENTION_EVERYONE, 1 << 17);
        assert_eq!(permissions::USE_EXTERNAL_EMOJIS, 1 << 18);
        assert_eq!(permissions::VIEW_GUILD_INSIGHTS, 1 << 19);
        assert_eq!(permissions::CONNECT, 1 << 20);
        assert_eq!(permissions::SPEAK, 1 << 21);
        assert_eq!(permissions::MUTE_MEMBERS, 1 << 22);
        assert_eq!(permissions::DEAFEN_MEMBERS, 1 << 23);
        assert_eq!(permissions::MOVE_MEMBERS, 1 << 24);
        assert_eq!(permissions::USE_VAD, 1 << 25);
        assert_eq!(permissions::CHANGE_NICKNAME, 1 << 26);
        assert_eq!(permissions::MANAGE_NICKNAMES, 1 << 27);
        assert_eq!(permissions::MANAGE_ROLES, 1 << 28);
        assert_eq!(permissions::MANAGE_WEBHOOKS, 1 << 29);
        assert_eq!(permissions::MANAGE_EMOJIS_AND_STICKERS, 1 << 30);
    }

    // ==========================================================================
    // Role Position Tests
    // ==========================================================================

    #[test]
    fn test_role_position_default_zero() {
        let role = Role::default();
        assert_eq!(role.position, 0);
    }

    #[test]
    fn test_role_position_ordering() {
        let mut roles = vec![
            {
                let mut r = create_test_role();
                r.position = 2;
                r.name = "role-c".to_string();
                r
            },
            {
                let mut r = create_test_role();
                r.position = 0;
                r.name = "role-a".to_string();
                r
            },
            {
                let mut r = create_test_role();
                r.position = 1;
                r.name = "role-b".to_string();
                r
            },
        ];

        roles.sort_by_key(|r| r.position);

        assert_eq!(roles[0].name, "role-a");
        assert_eq!(roles[1].name, "role-b");
        assert_eq!(roles[2].name, "role-c");
    }

    // ==========================================================================
    // Role Hoist Tests
    // ==========================================================================

    #[test]
    fn test_role_hoist_default_false() {
        let role = create_test_role();
        assert!(!role.hoist);
    }

    #[test]
    fn test_role_hoist_can_be_set() {
        let mut role = create_test_role();
        role.hoist = true;
        assert!(role.hoist);
    }

    // ==========================================================================
    // Role Mentionable Tests
    // ==========================================================================

    #[test]
    fn test_role_mentionable_default_false() {
        let role = create_test_role();
        assert!(!role.mentionable);
    }

    #[test]
    fn test_role_mentionable_can_be_set() {
        let mut role = create_test_role();
        role.mentionable = true;
        assert!(role.mentionable);
    }

    // ==========================================================================
    // Role Clone Tests
    // ==========================================================================

    #[test]
    fn test_role_clone() {
        let mut role = create_test_role();
        role.permissions = permissions::VIEW_CHANNEL | permissions::SEND_MESSAGES;
        role.color = Some(0xFF5733);

        let cloned = role.clone();

        assert_eq!(role.id, cloned.id);
        assert_eq!(role.server_id, cloned.server_id);
        assert_eq!(role.name, cloned.name);
        assert_eq!(role.permissions, cloned.permissions);
        assert_eq!(role.position, cloned.position);
        assert_eq!(role.color, cloned.color);
        assert_eq!(role.hoist, cloned.hoist);
        assert_eq!(role.mentionable, cloned.mentionable);
    }

    // ==========================================================================
    // Role Serialization Tests
    // ==========================================================================

    #[test]
    fn test_role_serialization() {
        let mut role = create_test_role();
        role.color = Some(0xFF5733);
        role.hoist = true;

        let serialized = serde_json::to_string(&role).expect("Failed to serialize role");

        assert!(serialized.contains("\"id\":12345678901234567"));
        assert!(serialized.contains("\"server_id\":100"));
        assert!(serialized.contains("\"name\":\"Test Role\""));
        assert!(serialized.contains("\"hoist\":true"));
    }

    // ==========================================================================
    // Everyone Role Tests
    // ==========================================================================

    #[test]
    fn test_everyone_role_has_same_id_as_server() {
        // In Discord, the @everyone role has the same ID as the server
        let server_id = 100_i64;
        let mut everyone_role = create_test_role();
        everyone_role.id = server_id;
        everyone_role.server_id = server_id;
        everyone_role.name = "@everyone".to_string();

        assert_eq!(everyone_role.id, everyone_role.server_id);
    }
}
