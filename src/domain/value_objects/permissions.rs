//! Discord-compatible permission flags.
//!
//! Permissions are represented as a 64-bit bitfield where each bit
//! represents a specific permission.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 64-bit permission bitfield.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Permissions(pub i64);

impl Permissions {
    // General permissions
    /// Allows creation of instant invites
    pub const CREATE_INSTANT_INVITE: i64 = 1 << 0;
    /// Allows kicking members
    pub const KICK_MEMBERS: i64 = 1 << 1;
    /// Allows banning members
    pub const BAN_MEMBERS: i64 = 1 << 2;
    /// Allows all permissions and bypasses channel permission overwrites
    pub const ADMINISTRATOR: i64 = 1 << 3;
    /// Allows management and editing of channels
    pub const MANAGE_CHANNELS: i64 = 1 << 4;
    /// Allows management and editing of the guild
    pub const MANAGE_GUILD: i64 = 1 << 5;
    /// Allows for the addition of reactions to messages
    pub const ADD_REACTIONS: i64 = 1 << 6;
    /// Allows for viewing of audit logs
    pub const VIEW_AUDIT_LOG: i64 = 1 << 7;
    /// Allows for using priority speaker in a voice channel
    pub const PRIORITY_SPEAKER: i64 = 1 << 8;
    /// Allows the user to go live
    pub const STREAM: i64 = 1 << 9;
    /// Allows guild members to view a channel
    pub const VIEW_CHANNEL: i64 = 1 << 10;
    /// Allows for sending messages in a channel
    pub const SEND_MESSAGES: i64 = 1 << 11;
    /// Allows for sending TTS messages
    pub const SEND_TTS_MESSAGES: i64 = 1 << 12;
    /// Allows for deletion of other users messages
    pub const MANAGE_MESSAGES: i64 = 1 << 13;
    /// Links sent by users with this permission will be auto-embedded
    pub const EMBED_LINKS: i64 = 1 << 14;
    /// Allows for uploading images and files
    pub const ATTACH_FILES: i64 = 1 << 15;
    /// Allows for reading of message history
    pub const READ_MESSAGE_HISTORY: i64 = 1 << 16;
    /// Allows for using the @everyone tag and @here tag
    pub const MENTION_EVERYONE: i64 = 1 << 17;
    /// Allows the usage of custom emojis from other servers
    pub const USE_EXTERNAL_EMOJIS: i64 = 1 << 18;
    /// Allows for viewing guild insights
    pub const VIEW_GUILD_INSIGHTS: i64 = 1 << 19;
    /// Allows for joining of a voice channel
    pub const CONNECT: i64 = 1 << 20;
    /// Allows for speaking in a voice channel
    pub const SPEAK: i64 = 1 << 21;
    /// Allows for muting members in a voice channel
    pub const MUTE_MEMBERS: i64 = 1 << 22;
    /// Allows for deafening of members in a voice channel
    pub const DEAFEN_MEMBERS: i64 = 1 << 23;
    /// Allows for moving of members between voice channels
    pub const MOVE_MEMBERS: i64 = 1 << 24;
    /// Allows for using voice-activity-detection in a voice channel
    pub const USE_VAD: i64 = 1 << 25;
    /// Allows for modification of own nickname
    pub const CHANGE_NICKNAME: i64 = 1 << 26;
    /// Allows for modification of other users nicknames
    pub const MANAGE_NICKNAMES: i64 = 1 << 27;
    /// Allows management and editing of roles
    pub const MANAGE_ROLES: i64 = 1 << 28;
    /// Allows management and editing of webhooks
    pub const MANAGE_WEBHOOKS: i64 = 1 << 29;
    /// Allows management and editing of emojis and stickers
    pub const MANAGE_EMOJIS_AND_STICKERS: i64 = 1 << 30;
    /// Allows members to use application commands
    pub const USE_APPLICATION_COMMANDS: i64 = 1 << 31;
    /// Allows for requesting to speak in stage channels
    pub const REQUEST_TO_SPEAK: i64 = 1 << 32;
    /// Allows for creating, editing, and deleting scheduled events
    pub const MANAGE_EVENTS: i64 = 1 << 33;
    /// Allows for deleting and archiving threads, and viewing all private threads
    pub const MANAGE_THREADS: i64 = 1 << 34;
    /// Allows for creating public and announcement threads
    pub const CREATE_PUBLIC_THREADS: i64 = 1 << 35;
    /// Allows for creating private threads
    pub const CREATE_PRIVATE_THREADS: i64 = 1 << 36;
    /// Allows the usage of custom stickers from other servers
    pub const USE_EXTERNAL_STICKERS: i64 = 1 << 37;
    /// Allows for sending messages in threads
    pub const SEND_MESSAGES_IN_THREADS: i64 = 1 << 38;
    /// Allows for using Activities (applications with EMBEDDED flag)
    pub const USE_EMBEDDED_ACTIVITIES: i64 = 1 << 39;
    /// Allows for timing out users
    pub const MODERATE_MEMBERS: i64 = 1 << 40;

    /// All permissions combined
    pub const ALL: i64 = 0x1FFFFFFFFFF;

    /// Default permissions for @everyone role
    pub const DEFAULT: i64 = Self::VIEW_CHANNEL
        | Self::CREATE_INSTANT_INVITE
        | Self::SEND_MESSAGES
        | Self::EMBED_LINKS
        | Self::ATTACH_FILES
        | Self::READ_MESSAGE_HISTORY
        | Self::ADD_REACTIONS
        | Self::USE_EXTERNAL_EMOJIS
        | Self::CONNECT
        | Self::SPEAK
        | Self::USE_VAD
        | Self::CHANGE_NICKNAME;

    /// Create a new Permissions instance.
    pub const fn new(bits: i64) -> Self {
        Self(bits)
    }

    /// Create empty permissions.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create permissions with all flags set.
    pub const fn all() -> Self {
        Self(Self::ALL)
    }

    /// Check if a specific permission is set.
    pub const fn has(&self, permission: i64) -> bool {
        // Administrator overrides all
        if self.0 & Self::ADMINISTRATOR != 0 {
            return true;
        }
        self.0 & permission == permission
    }

    /// Check if administrator permission is set.
    pub const fn is_admin(&self) -> bool {
        self.0 & Self::ADMINISTRATOR != 0
    }

    /// Add a permission.
    pub fn add(&mut self, permission: i64) {
        self.0 |= permission;
    }

    /// Remove a permission.
    pub fn remove(&mut self, permission: i64) {
        self.0 &= !permission;
    }

    /// Toggle a permission.
    pub fn toggle(&mut self, permission: i64) {
        self.0 ^= permission;
    }

    /// Combine with another Permissions (union).
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Get intersection with another Permissions.
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Get the raw bits.
    pub const fn bits(&self) -> i64 {
        self.0
    }

    /// Compute effective permissions after applying overwrites.
    ///
    /// # Arguments
    /// * `base` - Base permissions (from roles)
    /// * `allow` - Permissions to allow (from overwrite)
    /// * `deny` - Permissions to deny (from overwrite)
    pub fn apply_overwrites(base: i64, allow: i64, deny: i64) -> i64 {
        (base & !deny) | allow
    }
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for Permissions {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Permissions> for i64 {
    fn from(perms: Permissions) -> Self {
        perms.0
    }
}

impl std::ops::BitOr for Permissions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Permissions {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_permission() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert!(!perms.has(Permissions::ADMINISTRATOR));
    }

    #[test]
    fn test_admin_overrides() {
        let perms = Permissions::new(Permissions::ADMINISTRATOR);

        // Admin should have all permissions
        assert!(perms.has(Permissions::MANAGE_GUILD));
        assert!(perms.has(Permissions::BAN_MEMBERS));
        assert!(perms.has(Permissions::MANAGE_CHANNELS));
    }

    #[test]
    fn test_apply_overwrites() {
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let allow = Permissions::MANAGE_MESSAGES;
        let deny = Permissions::SEND_MESSAGES;

        let result = Permissions::apply_overwrites(base, allow, deny);

        // Should have VIEW_CHANNEL (from base, not denied)
        assert!(result & Permissions::VIEW_CHANNEL != 0);
        // Should NOT have SEND_MESSAGES (denied)
        assert!(result & Permissions::SEND_MESSAGES == 0);
        // Should have MANAGE_MESSAGES (allowed)
        assert!(result & Permissions::MANAGE_MESSAGES != 0);
    }
}
