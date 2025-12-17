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

    // ==========================================================================
    // Basic Construction Tests
    // ==========================================================================

    #[test]
    fn test_permissions_new_creates_from_bits() {
        let bits = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let perms = Permissions::new(bits);

        assert_eq!(perms.bits(), bits);
    }

    #[test]
    fn test_permissions_empty_has_no_permissions() {
        let perms = Permissions::empty();

        assert_eq!(perms.bits(), 0);
        assert!(!perms.has(Permissions::VIEW_CHANNEL));
        assert!(!perms.has(Permissions::SEND_MESSAGES));
        assert!(!perms.is_admin());
    }

    #[test]
    fn test_permissions_all_has_all_permissions() {
        let perms = Permissions::all();

        assert_eq!(perms.bits(), Permissions::ALL);
        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert!(perms.has(Permissions::ADMINISTRATOR));
        assert!(perms.has(Permissions::MODERATE_MEMBERS));
    }

    #[test]
    fn test_permissions_default_is_empty() {
        let perms = Permissions::default();
        assert_eq!(perms.bits(), 0);
    }

    // ==========================================================================
    // Permission Flag Tests - All Discord-compatible flags
    // ==========================================================================

    #[test]
    fn test_permission_flag_create_instant_invite() {
        let perms = Permissions::new(Permissions::CREATE_INSTANT_INVITE);
        assert!(perms.has(Permissions::CREATE_INSTANT_INVITE));
        assert_eq!(Permissions::CREATE_INSTANT_INVITE, 1 << 0);
    }

    #[test]
    fn test_permission_flag_kick_members() {
        let perms = Permissions::new(Permissions::KICK_MEMBERS);
        assert!(perms.has(Permissions::KICK_MEMBERS));
        assert_eq!(Permissions::KICK_MEMBERS, 1 << 1);
    }

    #[test]
    fn test_permission_flag_ban_members() {
        let perms = Permissions::new(Permissions::BAN_MEMBERS);
        assert!(perms.has(Permissions::BAN_MEMBERS));
        assert_eq!(Permissions::BAN_MEMBERS, 1 << 2);
    }

    #[test]
    fn test_permission_flag_administrator() {
        let perms = Permissions::new(Permissions::ADMINISTRATOR);
        assert!(perms.has(Permissions::ADMINISTRATOR));
        assert!(perms.is_admin());
        assert_eq!(Permissions::ADMINISTRATOR, 1 << 3);
    }

    #[test]
    fn test_permission_flag_manage_channels() {
        let perms = Permissions::new(Permissions::MANAGE_CHANNELS);
        assert!(perms.has(Permissions::MANAGE_CHANNELS));
        assert_eq!(Permissions::MANAGE_CHANNELS, 1 << 4);
    }

    #[test]
    fn test_permission_flag_manage_guild() {
        let perms = Permissions::new(Permissions::MANAGE_GUILD);
        assert!(perms.has(Permissions::MANAGE_GUILD));
        assert_eq!(Permissions::MANAGE_GUILD, 1 << 5);
    }

    #[test]
    fn test_permission_flag_add_reactions() {
        let perms = Permissions::new(Permissions::ADD_REACTIONS);
        assert!(perms.has(Permissions::ADD_REACTIONS));
        assert_eq!(Permissions::ADD_REACTIONS, 1 << 6);
    }

    #[test]
    fn test_permission_flag_view_audit_log() {
        let perms = Permissions::new(Permissions::VIEW_AUDIT_LOG);
        assert!(perms.has(Permissions::VIEW_AUDIT_LOG));
        assert_eq!(Permissions::VIEW_AUDIT_LOG, 1 << 7);
    }

    #[test]
    fn test_permission_flag_priority_speaker() {
        let perms = Permissions::new(Permissions::PRIORITY_SPEAKER);
        assert!(perms.has(Permissions::PRIORITY_SPEAKER));
        assert_eq!(Permissions::PRIORITY_SPEAKER, 1 << 8);
    }

    #[test]
    fn test_permission_flag_stream() {
        let perms = Permissions::new(Permissions::STREAM);
        assert!(perms.has(Permissions::STREAM));
        assert_eq!(Permissions::STREAM, 1 << 9);
    }

    #[test]
    fn test_permission_flag_view_channel() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL);
        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert_eq!(Permissions::VIEW_CHANNEL, 1 << 10);
    }

    #[test]
    fn test_permission_flag_send_messages() {
        let perms = Permissions::new(Permissions::SEND_MESSAGES);
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert_eq!(Permissions::SEND_MESSAGES, 1 << 11);
    }

    #[test]
    fn test_permission_flag_send_tts_messages() {
        let perms = Permissions::new(Permissions::SEND_TTS_MESSAGES);
        assert!(perms.has(Permissions::SEND_TTS_MESSAGES));
        assert_eq!(Permissions::SEND_TTS_MESSAGES, 1 << 12);
    }

    #[test]
    fn test_permission_flag_manage_messages() {
        let perms = Permissions::new(Permissions::MANAGE_MESSAGES);
        assert!(perms.has(Permissions::MANAGE_MESSAGES));
        assert_eq!(Permissions::MANAGE_MESSAGES, 1 << 13);
    }

    #[test]
    fn test_permission_flag_embed_links() {
        let perms = Permissions::new(Permissions::EMBED_LINKS);
        assert!(perms.has(Permissions::EMBED_LINKS));
        assert_eq!(Permissions::EMBED_LINKS, 1 << 14);
    }

    #[test]
    fn test_permission_flag_attach_files() {
        let perms = Permissions::new(Permissions::ATTACH_FILES);
        assert!(perms.has(Permissions::ATTACH_FILES));
        assert_eq!(Permissions::ATTACH_FILES, 1 << 15);
    }

    #[test]
    fn test_permission_flag_read_message_history() {
        let perms = Permissions::new(Permissions::READ_MESSAGE_HISTORY);
        assert!(perms.has(Permissions::READ_MESSAGE_HISTORY));
        assert_eq!(Permissions::READ_MESSAGE_HISTORY, 1 << 16);
    }

    #[test]
    fn test_permission_flag_mention_everyone() {
        let perms = Permissions::new(Permissions::MENTION_EVERYONE);
        assert!(perms.has(Permissions::MENTION_EVERYONE));
        assert_eq!(Permissions::MENTION_EVERYONE, 1 << 17);
    }

    #[test]
    fn test_permission_flag_use_external_emojis() {
        let perms = Permissions::new(Permissions::USE_EXTERNAL_EMOJIS);
        assert!(perms.has(Permissions::USE_EXTERNAL_EMOJIS));
        assert_eq!(Permissions::USE_EXTERNAL_EMOJIS, 1 << 18);
    }

    #[test]
    fn test_permission_flag_view_guild_insights() {
        let perms = Permissions::new(Permissions::VIEW_GUILD_INSIGHTS);
        assert!(perms.has(Permissions::VIEW_GUILD_INSIGHTS));
        assert_eq!(Permissions::VIEW_GUILD_INSIGHTS, 1 << 19);
    }

    #[test]
    fn test_permission_flag_connect() {
        let perms = Permissions::new(Permissions::CONNECT);
        assert!(perms.has(Permissions::CONNECT));
        assert_eq!(Permissions::CONNECT, 1 << 20);
    }

    #[test]
    fn test_permission_flag_speak() {
        let perms = Permissions::new(Permissions::SPEAK);
        assert!(perms.has(Permissions::SPEAK));
        assert_eq!(Permissions::SPEAK, 1 << 21);
    }

    #[test]
    fn test_permission_flag_mute_members() {
        let perms = Permissions::new(Permissions::MUTE_MEMBERS);
        assert!(perms.has(Permissions::MUTE_MEMBERS));
        assert_eq!(Permissions::MUTE_MEMBERS, 1 << 22);
    }

    #[test]
    fn test_permission_flag_deafen_members() {
        let perms = Permissions::new(Permissions::DEAFEN_MEMBERS);
        assert!(perms.has(Permissions::DEAFEN_MEMBERS));
        assert_eq!(Permissions::DEAFEN_MEMBERS, 1 << 23);
    }

    #[test]
    fn test_permission_flag_move_members() {
        let perms = Permissions::new(Permissions::MOVE_MEMBERS);
        assert!(perms.has(Permissions::MOVE_MEMBERS));
        assert_eq!(Permissions::MOVE_MEMBERS, 1 << 24);
    }

    #[test]
    fn test_permission_flag_use_vad() {
        let perms = Permissions::new(Permissions::USE_VAD);
        assert!(perms.has(Permissions::USE_VAD));
        assert_eq!(Permissions::USE_VAD, 1 << 25);
    }

    #[test]
    fn test_permission_flag_change_nickname() {
        let perms = Permissions::new(Permissions::CHANGE_NICKNAME);
        assert!(perms.has(Permissions::CHANGE_NICKNAME));
        assert_eq!(Permissions::CHANGE_NICKNAME, 1 << 26);
    }

    #[test]
    fn test_permission_flag_manage_nicknames() {
        let perms = Permissions::new(Permissions::MANAGE_NICKNAMES);
        assert!(perms.has(Permissions::MANAGE_NICKNAMES));
        assert_eq!(Permissions::MANAGE_NICKNAMES, 1 << 27);
    }

    #[test]
    fn test_permission_flag_manage_roles() {
        let perms = Permissions::new(Permissions::MANAGE_ROLES);
        assert!(perms.has(Permissions::MANAGE_ROLES));
        assert_eq!(Permissions::MANAGE_ROLES, 1 << 28);
    }

    #[test]
    fn test_permission_flag_manage_webhooks() {
        let perms = Permissions::new(Permissions::MANAGE_WEBHOOKS);
        assert!(perms.has(Permissions::MANAGE_WEBHOOKS));
        assert_eq!(Permissions::MANAGE_WEBHOOKS, 1 << 29);
    }

    #[test]
    fn test_permission_flag_manage_emojis_and_stickers() {
        let perms = Permissions::new(Permissions::MANAGE_EMOJIS_AND_STICKERS);
        assert!(perms.has(Permissions::MANAGE_EMOJIS_AND_STICKERS));
        assert_eq!(Permissions::MANAGE_EMOJIS_AND_STICKERS, 1 << 30);
    }

    #[test]
    fn test_permission_flag_use_application_commands() {
        let perms = Permissions::new(Permissions::USE_APPLICATION_COMMANDS);
        assert!(perms.has(Permissions::USE_APPLICATION_COMMANDS));
        assert_eq!(Permissions::USE_APPLICATION_COMMANDS, 1 << 31);
    }

    #[test]
    fn test_permission_flag_request_to_speak() {
        let perms = Permissions::new(Permissions::REQUEST_TO_SPEAK);
        assert!(perms.has(Permissions::REQUEST_TO_SPEAK));
        assert_eq!(Permissions::REQUEST_TO_SPEAK, 1 << 32);
    }

    #[test]
    fn test_permission_flag_manage_events() {
        let perms = Permissions::new(Permissions::MANAGE_EVENTS);
        assert!(perms.has(Permissions::MANAGE_EVENTS));
        assert_eq!(Permissions::MANAGE_EVENTS, 1 << 33);
    }

    #[test]
    fn test_permission_flag_manage_threads() {
        let perms = Permissions::new(Permissions::MANAGE_THREADS);
        assert!(perms.has(Permissions::MANAGE_THREADS));
        assert_eq!(Permissions::MANAGE_THREADS, 1 << 34);
    }

    #[test]
    fn test_permission_flag_create_public_threads() {
        let perms = Permissions::new(Permissions::CREATE_PUBLIC_THREADS);
        assert!(perms.has(Permissions::CREATE_PUBLIC_THREADS));
        assert_eq!(Permissions::CREATE_PUBLIC_THREADS, 1 << 35);
    }

    #[test]
    fn test_permission_flag_create_private_threads() {
        let perms = Permissions::new(Permissions::CREATE_PRIVATE_THREADS);
        assert!(perms.has(Permissions::CREATE_PRIVATE_THREADS));
        assert_eq!(Permissions::CREATE_PRIVATE_THREADS, 1 << 36);
    }

    #[test]
    fn test_permission_flag_use_external_stickers() {
        let perms = Permissions::new(Permissions::USE_EXTERNAL_STICKERS);
        assert!(perms.has(Permissions::USE_EXTERNAL_STICKERS));
        assert_eq!(Permissions::USE_EXTERNAL_STICKERS, 1 << 37);
    }

    #[test]
    fn test_permission_flag_send_messages_in_threads() {
        let perms = Permissions::new(Permissions::SEND_MESSAGES_IN_THREADS);
        assert!(perms.has(Permissions::SEND_MESSAGES_IN_THREADS));
        assert_eq!(Permissions::SEND_MESSAGES_IN_THREADS, 1 << 38);
    }

    #[test]
    fn test_permission_flag_use_embedded_activities() {
        let perms = Permissions::new(Permissions::USE_EMBEDDED_ACTIVITIES);
        assert!(perms.has(Permissions::USE_EMBEDDED_ACTIVITIES));
        assert_eq!(Permissions::USE_EMBEDDED_ACTIVITIES, 1 << 39);
    }

    #[test]
    fn test_permission_flag_moderate_members() {
        let perms = Permissions::new(Permissions::MODERATE_MEMBERS);
        assert!(perms.has(Permissions::MODERATE_MEMBERS));
        assert_eq!(Permissions::MODERATE_MEMBERS, 1 << 40);
    }

    // ==========================================================================
    // Has Permission Tests
    // ==========================================================================

    #[test]
    fn test_has_permission_single() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL);
        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(!perms.has(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_has_permission_multiple() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert!(!perms.has(Permissions::ADMINISTRATOR));
    }

    #[test]
    fn test_has_permission_combined_check() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        // Check for multiple permissions at once
        let required = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        assert!(perms.has(required));

        // Fails if any required permission is missing
        let too_many = Permissions::VIEW_CHANNEL | Permissions::ADMINISTRATOR;
        assert!(!perms.has(too_many));
    }

    // ==========================================================================
    // Administrator Override Tests
    // ==========================================================================

    #[test]
    fn test_admin_has_all_permissions() {
        let perms = Permissions::new(Permissions::ADMINISTRATOR);

        // Admin should have all permissions
        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert!(perms.has(Permissions::MANAGE_GUILD));
        assert!(perms.has(Permissions::BAN_MEMBERS));
        assert!(perms.has(Permissions::MANAGE_CHANNELS));
        assert!(perms.has(Permissions::MANAGE_ROLES));
        assert!(perms.has(Permissions::MODERATE_MEMBERS));
    }

    #[test]
    fn test_admin_override_without_explicit_permission() {
        // Admin only has ADMINISTRATOR bit set, not VIEW_CHANNEL
        let perms = Permissions::new(Permissions::ADMINISTRATOR);

        // But should still return true for has(VIEW_CHANNEL)
        assert!(perms.has(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn test_is_admin_true_when_has_administrator() {
        let perms = Permissions::new(Permissions::ADMINISTRATOR);
        assert!(perms.is_admin());
    }

    #[test]
    fn test_is_admin_false_when_no_administrator() {
        let perms = Permissions::new(Permissions::MANAGE_GUILD | Permissions::BAN_MEMBERS);
        assert!(!perms.is_admin());
    }

    // ==========================================================================
    // Add Permission Tests
    // ==========================================================================

    #[test]
    fn test_add_permission() {
        let mut perms = Permissions::empty();

        perms.add(Permissions::VIEW_CHANNEL);
        assert!(perms.has(Permissions::VIEW_CHANNEL));

        perms.add(Permissions::SEND_MESSAGES);
        assert!(perms.has(Permissions::SEND_MESSAGES));
        assert!(perms.has(Permissions::VIEW_CHANNEL)); // Still has previous
    }

    #[test]
    fn test_add_permission_idempotent() {
        let mut perms = Permissions::new(Permissions::VIEW_CHANNEL);
        let original_bits = perms.bits();

        perms.add(Permissions::VIEW_CHANNEL);

        assert_eq!(perms.bits(), original_bits);
    }

    #[test]
    fn test_add_multiple_permissions() {
        let mut perms = Permissions::empty();

        perms.add(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(perms.has(Permissions::SEND_MESSAGES));
    }

    // ==========================================================================
    // Remove Permission Tests
    // ==========================================================================

    #[test]
    fn test_remove_permission() {
        let mut perms = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        perms.remove(Permissions::SEND_MESSAGES);

        assert!(perms.has(Permissions::VIEW_CHANNEL));
        assert!(!perms.has(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_remove_permission_idempotent() {
        let mut perms = Permissions::new(Permissions::VIEW_CHANNEL);
        let original_bits = perms.bits();

        perms.remove(Permissions::SEND_MESSAGES); // Remove permission that doesn't exist

        assert_eq!(perms.bits(), original_bits);
    }

    #[test]
    fn test_remove_multiple_permissions() {
        let mut perms = Permissions::new(
            Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES
        );

        perms.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);

        assert!(!perms.has(Permissions::VIEW_CHANNEL));
        assert!(!perms.has(Permissions::SEND_MESSAGES));
        assert!(perms.has(Permissions::MANAGE_MESSAGES));
    }

    // ==========================================================================
    // Toggle Permission Tests
    // ==========================================================================

    #[test]
    fn test_toggle_adds_permission_when_absent() {
        let mut perms = Permissions::empty();

        perms.toggle(Permissions::VIEW_CHANNEL);

        assert!(perms.has(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn test_toggle_removes_permission_when_present() {
        let mut perms = Permissions::new(Permissions::VIEW_CHANNEL);

        perms.toggle(Permissions::VIEW_CHANNEL);

        // Note: has() with admin override would still return false for VIEW_CHANNEL
        // since ADMINISTRATOR is not set
        assert_eq!(perms.bits() & Permissions::VIEW_CHANNEL, 0);
    }

    #[test]
    fn test_toggle_twice_returns_to_original() {
        let mut perms = Permissions::new(Permissions::SEND_MESSAGES);
        let original = perms.bits();

        perms.toggle(Permissions::VIEW_CHANNEL);
        perms.toggle(Permissions::VIEW_CHANNEL);

        assert_eq!(perms.bits(), original);
    }

    // ==========================================================================
    // Union Tests
    // ==========================================================================

    #[test]
    fn test_union_combines_permissions() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES);

        let combined = p1.union(p2);

        assert!(combined.has(Permissions::VIEW_CHANNEL));
        assert!(combined.has(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_union_with_empty() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = Permissions::empty();

        let combined = p1.union(p2);

        assert_eq!(combined.bits(), p1.bits());
    }

    #[test]
    fn test_union_with_overlapping() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES);

        let combined = p1.union(p2);

        assert!(combined.has(Permissions::VIEW_CHANNEL));
        assert!(combined.has(Permissions::SEND_MESSAGES));
        assert!(combined.has(Permissions::MANAGE_MESSAGES));
    }

    // ==========================================================================
    // Intersection Tests
    // ==========================================================================

    #[test]
    fn test_intersection_finds_common_permissions() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES);

        let common = p1.intersection(p2);

        assert!(!common.has(Permissions::VIEW_CHANNEL)); // Only in p1
        assert!(common.bits() & Permissions::SEND_MESSAGES != 0); // In both
        assert!(!common.has(Permissions::MANAGE_MESSAGES)); // Only in p2
    }

    #[test]
    fn test_intersection_with_no_overlap() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES);

        let common = p1.intersection(p2);

        assert_eq!(common.bits(), 0);
    }

    // ==========================================================================
    // Apply Overwrites Tests
    // ==========================================================================

    #[test]
    fn test_apply_overwrites_allow() {
        let base = Permissions::VIEW_CHANNEL;
        let allow = Permissions::SEND_MESSAGES;
        let deny = 0;

        let result = Permissions::apply_overwrites(base, allow, deny);

        assert!(result & Permissions::VIEW_CHANNEL != 0);
        assert!(result & Permissions::SEND_MESSAGES != 0);
    }

    #[test]
    fn test_apply_overwrites_deny() {
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let allow = 0;
        let deny = Permissions::SEND_MESSAGES;

        let result = Permissions::apply_overwrites(base, allow, deny);

        assert!(result & Permissions::VIEW_CHANNEL != 0);
        assert!(result & Permissions::SEND_MESSAGES == 0);
    }

    #[test]
    fn test_apply_overwrites_allow_overrides_deny() {
        // When the same permission is both allowed and denied, allow wins
        // because we apply deny first, then allow
        let base = 0;
        let allow = Permissions::SEND_MESSAGES;
        let deny = Permissions::SEND_MESSAGES;

        let result = Permissions::apply_overwrites(base, allow, deny);

        // Formula: (base & !deny) | allow
        // (0 & !SEND) | SEND = 0 | SEND = SEND
        assert!(result & Permissions::SEND_MESSAGES != 0);
    }

    #[test]
    fn test_apply_overwrites_complex() {
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let allow = Permissions::MANAGE_MESSAGES;
        let deny = Permissions::SEND_MESSAGES;

        let result = Permissions::apply_overwrites(base, allow, deny);

        // VIEW_CHANNEL: from base, not denied
        assert!(result & Permissions::VIEW_CHANNEL != 0);
        // SEND_MESSAGES: was in base but denied
        assert!(result & Permissions::SEND_MESSAGES == 0);
        // MANAGE_MESSAGES: allowed
        assert!(result & Permissions::MANAGE_MESSAGES != 0);
    }

    #[test]
    fn test_apply_overwrites_no_changes() {
        let base = Permissions::VIEW_CHANNEL;
        let allow = 0;
        let deny = 0;

        let result = Permissions::apply_overwrites(base, allow, deny);

        assert_eq!(result, base);
    }

    // ==========================================================================
    // Default Permissions Tests
    // ==========================================================================

    #[test]
    fn test_default_permissions_include_basic_text() {
        let default = Permissions::DEFAULT;

        assert!(default & Permissions::VIEW_CHANNEL != 0);
        assert!(default & Permissions::SEND_MESSAGES != 0);
        assert!(default & Permissions::READ_MESSAGE_HISTORY != 0);
        assert!(default & Permissions::EMBED_LINKS != 0);
        assert!(default & Permissions::ATTACH_FILES != 0);
    }

    #[test]
    fn test_default_permissions_include_basic_voice() {
        let default = Permissions::DEFAULT;

        assert!(default & Permissions::CONNECT != 0);
        assert!(default & Permissions::SPEAK != 0);
        assert!(default & Permissions::USE_VAD != 0);
    }

    #[test]
    fn test_default_permissions_exclude_moderation() {
        let default = Permissions::DEFAULT;

        assert!(default & Permissions::ADMINISTRATOR == 0);
        assert!(default & Permissions::KICK_MEMBERS == 0);
        assert!(default & Permissions::BAN_MEMBERS == 0);
        assert!(default & Permissions::MANAGE_CHANNELS == 0);
        assert!(default & Permissions::MANAGE_GUILD == 0);
        assert!(default & Permissions::MANAGE_MESSAGES == 0);
        assert!(default & Permissions::MANAGE_ROLES == 0);
    }

    // ==========================================================================
    // Conversion Tests
    // ==========================================================================

    #[test]
    fn test_permissions_from_i64() {
        let bits = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let perms: Permissions = bits.into();

        assert_eq!(perms.bits(), bits);
    }

    #[test]
    fn test_permissions_into_i64() {
        let perms = Permissions::new(Permissions::VIEW_CHANNEL);
        let bits: i64 = perms.into();

        assert_eq!(bits, Permissions::VIEW_CHANNEL);
    }

    #[test]
    fn test_permissions_display() {
        let perms = Permissions::new(12345);
        let display = format!("{}", perms);

        assert_eq!(display, "12345");
    }

    // ==========================================================================
    // Bitwise Operator Tests
    // ==========================================================================

    #[test]
    fn test_bitor_operator() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES);

        let combined = p1 | p2;

        assert_eq!(combined.bits(), Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);
    }

    #[test]
    fn test_bitand_operator() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);
        let p2 = Permissions::new(Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES);

        let intersection = p1 & p2;

        assert_eq!(intersection.bits(), Permissions::SEND_MESSAGES);
    }

    // ==========================================================================
    // Clone and Copy Tests
    // ==========================================================================

    #[test]
    fn test_permissions_is_copy() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = p1; // Copy

        assert_eq!(p1, p2);
    }

    #[test]
    fn test_permissions_clone() {
        let p1 = Permissions::new(Permissions::VIEW_CHANNEL);
        let p2 = p1.clone();

        assert_eq!(p1, p2);
    }

    // ==========================================================================
    // Edge Cases
    // ==========================================================================

    #[test]
    fn test_permissions_with_zero() {
        let perms = Permissions::new(0);
        assert_eq!(perms.bits(), 0);
        assert!(!perms.has(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn test_permissions_with_negative_value() {
        // i64 can be negative; test that it works
        let perms = Permissions::new(-1);
        // -1 in two's complement has all bits set
        assert_eq!(perms.bits(), -1);
    }

    #[test]
    fn test_all_constant_covers_all_flags() {
        let all = Permissions::ALL;

        // All bits up to bit 40 should be set
        assert!(all & Permissions::CREATE_INSTANT_INVITE != 0);
        assert!(all & Permissions::MODERATE_MEMBERS != 0);
    }
}
