//! Permission calculation domain service.

use crate::domain::entities::{Channel, Member, Role, PermissionOverwrite};
use crate::domain::value_objects::Permissions;

/// Domain service for calculating and validating permissions.
pub struct PermissionService;

impl PermissionService {
    /// Calculate a member's base permissions in a server.
    ///
    /// This combines permissions from all roles the member has.
    pub fn calculate_base_permissions(member: &Member, roles: &[Role], owner_id: i64) -> i64 {
        // Owner has all permissions
        if member.user_id == owner_id {
            return Permissions::ALL;
        }

        let mut permissions = 0i64;

        for role in roles {
            if member.roles.contains(&role.id) || role.id == member.server_id {
                // Include @everyone role (same ID as server)
                permissions |= role.permissions;
            }
        }

        // Administrator overrides all
        if permissions & Permissions::ADMINISTRATOR != 0 {
            return Permissions::ALL;
        }

        permissions
    }

    /// Calculate a member's permissions in a specific channel.
    ///
    /// This applies channel permission overwrites to the base permissions.
    pub fn calculate_channel_permissions(
        member: &Member,
        _channel: &Channel,
        overwrites: &[PermissionOverwrite],
        roles: &[Role],
        owner_id: i64,
    ) -> i64 {
        // Start with base permissions
        let mut permissions = Self::calculate_base_permissions(member, roles, owner_id);

        // Administrator bypasses overwrites
        if permissions & Permissions::ADMINISTRATOR != 0 {
            return Permissions::ALL;
        }

        // Apply @everyone overwrites first
        for overwrite in overwrites {
            if overwrite.target_id == member.server_id && overwrite.target_type == "role" {
                permissions = Permissions::apply_overwrites(
                    permissions,
                    overwrite.allow,
                    overwrite.deny,
                );
            }
        }

        // Apply role overwrites
        let mut allow = 0i64;
        let mut deny = 0i64;

        for overwrite in overwrites {
            if overwrite.target_type == "role" && member.roles.contains(&overwrite.target_id) {
                allow |= overwrite.allow;
                deny |= overwrite.deny;
            }
        }

        permissions = Permissions::apply_overwrites(permissions, allow, deny);

        // Apply member-specific overwrites last
        for overwrite in overwrites {
            if overwrite.target_id == member.user_id && overwrite.target_type == "member" {
                permissions = Permissions::apply_overwrites(
                    permissions,
                    overwrite.allow,
                    overwrite.deny,
                );
            }
        }

        permissions
    }

    /// Check if a member can perform an action requiring specific permissions.
    pub fn can_perform(
        member: &Member,
        channel: &Channel,
        overwrites: &[PermissionOverwrite],
        roles: &[Role],
        owner_id: i64,
        required: i64,
    ) -> bool {
        let permissions = Self::calculate_channel_permissions(member, channel, overwrites, roles, owner_id);
        permissions & required == required
    }

    /// Check if a member can manage another member.
    ///
    /// A member can only manage others with lower role hierarchy.
    pub fn can_manage_member(
        actor: &Member,
        target: &Member,
        roles: &[Role],
        owner_id: i64,
    ) -> bool {
        // Owner can manage anyone
        if actor.user_id == owner_id {
            return true;
        }

        // Can't manage the owner
        if target.user_id == owner_id {
            return false;
        }

        // Get highest role position for both members
        let actor_highest = Self::highest_role_position(actor, roles);
        let target_highest = Self::highest_role_position(target, roles);

        actor_highest > target_highest
    }

    /// Get the highest role position for a member.
    fn highest_role_position(member: &Member, roles: &[Role]) -> i32 {
        roles
            .iter()
            .filter(|r| member.roles.contains(&r.id))
            .map(|r| r.position)
            .max()
            .unwrap_or(0)
    }

    /// Check if a member can modify a role.
    ///
    /// A member can only modify roles below their highest role.
    pub fn can_modify_role(member: &Member, role: &Role, roles: &[Role], owner_id: i64) -> bool {
        if member.user_id == owner_id {
            return true;
        }

        let member_highest = Self::highest_role_position(member, roles);
        role.position < member_highest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Test Helpers
    // ==========================================================================

    fn create_test_member(user_id: i64, server_id: i64, role_ids: Vec<i64>) -> Member {
        Member {
            user_id,
            server_id,
            roles: role_ids,
            ..Default::default()
        }
    }

    fn create_test_role(id: i64, server_id: i64, position: i32, permissions: i64) -> Role {
        Role {
            id,
            server_id,
            position,
            permissions,
            ..Default::default()
        }
    }

    fn create_test_channel(id: i64, server_id: i64) -> Channel {
        Channel {
            id,
            server_id: Some(server_id),
            ..Default::default()
        }
    }

    fn create_test_overwrite(
        channel_id: i64,
        target_id: i64,
        target_type: &str,
        allow: i64,
        deny: i64,
    ) -> PermissionOverwrite {
        PermissionOverwrite {
            channel_id,
            target_id,
            target_type: target_type.to_string(),
            allow,
            deny,
        }
    }

    // ==========================================================================
    // calculate_base_permissions Tests
    // ==========================================================================

    #[test]
    fn test_owner_has_all_permissions() {
        let member = create_test_member(1, 100, vec![]);
        let roles = vec![];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, Permissions::ALL);
    }

    #[test]
    fn test_owner_has_all_permissions_regardless_of_roles() {
        let member = create_test_member(1, 100, vec![101]);
        let roles = vec![create_test_role(101, 100, 1, 0)]; // Role with no permissions

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, Permissions::ALL);
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let member = create_test_member(2, 100, vec![101]);
        let roles = vec![create_test_role(101, 100, 1, Permissions::ADMINISTRATOR)];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, Permissions::ALL);
    }

    #[test]
    fn test_role_permissions_combine() {
        let member = create_test_member(2, 100, vec![101, 102]);
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
            create_test_role(102, 100, 2, Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
        assert!(perms & Permissions::SEND_MESSAGES != 0);
    }

    #[test]
    fn test_member_without_roles_has_no_permissions() {
        let member = create_test_member(2, 100, vec![]);
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, 0);
    }

    #[test]
    fn test_everyone_role_included() {
        // @everyone role has the same ID as the server
        let member = create_test_member(2, 100, vec![]); // No explicit roles
        let roles = vec![
            create_test_role(100, 100, 0, Permissions::VIEW_CHANNEL), // @everyone role
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
    }

    #[test]
    fn test_permissions_union_across_roles() {
        let member = create_test_member(2, 100, vec![101, 102, 103]);
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
            create_test_role(102, 100, 2, Permissions::SEND_MESSAGES),
            create_test_role(103, 100, 3, Permissions::MANAGE_MESSAGES),
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
        assert!(perms & Permissions::SEND_MESSAGES != 0);
        assert!(perms & Permissions::MANAGE_MESSAGES != 0);
    }

    #[test]
    fn test_admin_in_any_role_grants_all() {
        let member = create_test_member(2, 100, vec![101, 102]);
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
            create_test_role(102, 100, 2, Permissions::ADMINISTRATOR),
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, Permissions::ALL);
    }

    // ==========================================================================
    // calculate_channel_permissions Tests
    // ==========================================================================

    #[test]
    fn test_channel_permissions_owner_bypasses_overwrites() {
        let member = create_test_member(1, 100, vec![]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            create_test_overwrite(200, 1, "member", 0, Permissions::SEND_MESSAGES),
        ];
        let roles = vec![];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );
        assert_eq!(perms, Permissions::ALL);
    }

    #[test]
    fn test_channel_permissions_admin_bypasses_overwrites() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            create_test_overwrite(200, 2, "member", 0, Permissions::SEND_MESSAGES),
        ];
        let roles = vec![create_test_role(101, 100, 1, Permissions::ADMINISTRATOR)];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );
        assert_eq!(perms, Permissions::ALL);
    }

    #[test]
    fn test_channel_permissions_everyone_overwrite_applied() {
        let member = create_test_member(2, 100, vec![]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            // @everyone overwrite (target_id = server_id)
            create_test_overwrite(200, 100, "role", Permissions::ADD_REACTIONS, Permissions::SEND_MESSAGES),
        ];
        let roles = vec![
            create_test_role(100, 100, 0, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        // VIEW_CHANNEL from base (not denied)
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
        // SEND_MESSAGES denied by @everyone overwrite
        assert!(perms & Permissions::SEND_MESSAGES == 0);
        // ADD_REACTIONS allowed by @everyone overwrite
        assert!(perms & Permissions::ADD_REACTIONS != 0);
    }

    #[test]
    fn test_channel_permissions_role_overwrites_combined() {
        let member = create_test_member(2, 100, vec![101, 102]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            create_test_overwrite(200, 101, "role", Permissions::ADD_REACTIONS, 0),
            create_test_overwrite(200, 102, "role", Permissions::EMBED_LINKS, 0),
        ];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
            create_test_role(102, 100, 2, Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        assert!(perms & Permissions::ADD_REACTIONS != 0);
        assert!(perms & Permissions::EMBED_LINKS != 0);
    }

    #[test]
    fn test_channel_permissions_member_overwrite_last() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            // Role overwrite denies SEND_MESSAGES
            create_test_overwrite(200, 101, "role", 0, Permissions::SEND_MESSAGES),
            // Member overwrite allows SEND_MESSAGES (overrides role)
            create_test_overwrite(200, 2, "member", Permissions::SEND_MESSAGES, 0),
        ];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        // Member-specific overwrite should allow SEND_MESSAGES
        assert!(perms & Permissions::SEND_MESSAGES != 0);
    }

    #[test]
    fn test_channel_permissions_member_overwrite_deny() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![
            // Member overwrite denies SEND_MESSAGES
            create_test_overwrite(200, 2, "member", 0, Permissions::SEND_MESSAGES),
        ];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        // Member-specific overwrite denies SEND_MESSAGES
        assert!(perms & Permissions::SEND_MESSAGES == 0);
        // VIEW_CHANNEL still allowed
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
    }

    // ==========================================================================
    // can_perform Tests
    // ==========================================================================

    #[test]
    fn test_can_perform_true_when_has_permission() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let can = PermissionService::can_perform(
            &member, &channel, &overwrites, &roles, 1, Permissions::SEND_MESSAGES
        );
        assert!(can);
    }

    #[test]
    fn test_can_perform_false_when_missing_permission() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
        ];

        let can = PermissionService::can_perform(
            &member, &channel, &overwrites, &roles, 1, Permissions::SEND_MESSAGES
        );
        assert!(!can);
    }

    #[test]
    fn test_can_perform_multiple_permissions_required() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let required = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let can = PermissionService::can_perform(
            &member, &channel, &overwrites, &roles, 1, required
        );
        assert!(can);
    }

    #[test]
    fn test_can_perform_fails_when_partial_permissions() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites = vec![];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL), // Missing SEND_MESSAGES
        ];

        let required = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        let can = PermissionService::can_perform(
            &member, &channel, &overwrites, &roles, 1, required
        );
        assert!(!can);
    }

    // ==========================================================================
    // can_manage_member Tests
    // ==========================================================================

    #[test]
    fn test_can_manage_member_owner_can_manage_anyone() {
        let actor = create_test_member(1, 100, vec![]);
        let target = create_test_member(2, 100, vec![101]);
        let roles = vec![create_test_role(101, 100, 10, Permissions::ADMINISTRATOR)];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(can);
    }

    #[test]
    fn test_can_manage_member_cannot_manage_owner() {
        let actor = create_test_member(2, 100, vec![101]);
        let target = create_test_member(1, 100, vec![]); // Owner
        let roles = vec![create_test_role(101, 100, 10, Permissions::ADMINISTRATOR)];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(!can);
    }

    #[test]
    fn test_can_manage_member_higher_role_can_manage() {
        let actor = create_test_member(2, 100, vec![102]); // Higher position role
        let target = create_test_member(3, 100, vec![101]); // Lower position role
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::SEND_MESSAGES),
            create_test_role(102, 100, 5, Permissions::MANAGE_MESSAGES),
        ];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(can);
    }

    #[test]
    fn test_can_manage_member_lower_role_cannot_manage() {
        let actor = create_test_member(2, 100, vec![101]); // Lower position role
        let target = create_test_member(3, 100, vec![102]); // Higher position role
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::SEND_MESSAGES),
            create_test_role(102, 100, 5, Permissions::MANAGE_MESSAGES),
        ];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(!can);
    }

    #[test]
    fn test_can_manage_member_equal_roles_cannot_manage() {
        let actor = create_test_member(2, 100, vec![101]);
        let target = create_test_member(3, 100, vec![101]); // Same role
        let roles = vec![
            create_test_role(101, 100, 5, Permissions::MANAGE_MESSAGES),
        ];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(!can);
    }

    #[test]
    fn test_can_manage_member_with_multiple_roles() {
        let actor = create_test_member(2, 100, vec![101, 103]); // Roles at positions 1 and 7
        let target = create_test_member(3, 100, vec![102]); // Role at position 5
        let roles = vec![
            create_test_role(101, 100, 1, 0),
            create_test_role(102, 100, 5, 0),
            create_test_role(103, 100, 7, 0), // Actor's highest role
        ];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(can); // 7 > 5
    }

    // ==========================================================================
    // can_modify_role Tests
    // ==========================================================================

    #[test]
    fn test_can_modify_role_owner_can_modify_any() {
        let member = create_test_member(1, 100, vec![]);
        let role = create_test_role(101, 100, 10, Permissions::ADMINISTRATOR);
        let roles = vec![];

        let can = PermissionService::can_modify_role(&member, &role, &roles, 1);
        assert!(can);
    }

    #[test]
    fn test_can_modify_role_higher_position_can_modify() {
        let member = create_test_member(2, 100, vec![102]);
        let role = create_test_role(101, 100, 3, Permissions::SEND_MESSAGES);
        let roles = vec![
            create_test_role(102, 100, 5, Permissions::MANAGE_ROLES),
        ];

        let can = PermissionService::can_modify_role(&member, &role, &roles, 1);
        assert!(can); // Member's highest role (5) > target role (3)
    }

    #[test]
    fn test_can_modify_role_lower_position_cannot_modify() {
        let member = create_test_member(2, 100, vec![101]);
        let role = create_test_role(102, 100, 5, Permissions::MANAGE_ROLES);
        let roles = vec![
            create_test_role(101, 100, 3, Permissions::MANAGE_ROLES),
        ];

        let can = PermissionService::can_modify_role(&member, &role, &roles, 1);
        assert!(!can); // Member's highest role (3) < target role (5)
    }

    #[test]
    fn test_can_modify_role_equal_position_cannot_modify() {
        let member = create_test_member(2, 100, vec![101]);
        let role = create_test_role(102, 100, 5, Permissions::SEND_MESSAGES);
        let roles = vec![
            create_test_role(101, 100, 5, Permissions::MANAGE_ROLES),
        ];

        let can = PermissionService::can_modify_role(&member, &role, &roles, 1);
        assert!(!can); // Member's highest role (5) == target role (5)
    }

    // ==========================================================================
    // highest_role_position Tests (via public methods)
    // ==========================================================================

    #[test]
    fn test_highest_role_position_with_no_roles() {
        let actor = create_test_member(2, 100, vec![]);
        let target = create_test_member(3, 100, vec![101]);
        let roles = vec![
            create_test_role(101, 100, 1, 0),
        ];

        // Actor has no roles (position 0), target has role at position 1
        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(!can);
    }

    #[test]
    fn test_highest_role_position_multiple_roles_takes_max() {
        let actor = create_test_member(2, 100, vec![101, 102, 103]);
        let target = create_test_member(3, 100, vec![104]);
        let roles = vec![
            create_test_role(101, 100, 1, 0),
            create_test_role(102, 100, 3, 0),
            create_test_role(103, 100, 7, 0), // Actor's highest
            create_test_role(104, 100, 5, 0), // Target's role
        ];

        let can = PermissionService::can_manage_member(&actor, &target, &roles, 1);
        assert!(can); // Actor's highest (7) > Target's highest (5)
    }

    // ==========================================================================
    // Edge Cases
    // ==========================================================================

    #[test]
    fn test_member_with_nonexistent_role_ids() {
        let member = create_test_member(2, 100, vec![999]); // Role 999 doesn't exist
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL),
        ];

        let perms = PermissionService::calculate_base_permissions(&member, &roles, 1);
        assert_eq!(perms, 0); // No permissions because role doesn't exist
    }

    #[test]
    fn test_empty_overwrites_list() {
        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);
        let overwrites: Vec<PermissionOverwrite> = vec![];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        // Should just use base permissions
        assert!(perms & Permissions::VIEW_CHANNEL != 0);
        assert!(perms & Permissions::SEND_MESSAGES != 0);
    }

    #[test]
    fn test_overwrites_caller_responsible_for_filtering() {
        // Note: The PermissionService does not filter overwrites by channel_id.
        // The caller (repository layer) is expected to provide only overwrites
        // for the specific channel being queried.
        //
        // This test documents that behavior: if an overwrite for a role the member
        // has is passed in, it will be applied regardless of channel_id.

        let member = create_test_member(2, 100, vec![101]);
        let channel = create_test_channel(200, 100);

        // Even though this overwrite has a different channel_id (999),
        // it will still be applied because the service trusts the caller
        // to provide the correct overwrites.
        let overwrites = vec![
            create_test_overwrite(999, 101, "role", 0, Permissions::SEND_MESSAGES),
        ];
        let roles = vec![
            create_test_role(101, 100, 1, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        ];

        let perms = PermissionService::calculate_channel_permissions(
            &member, &channel, &overwrites, &roles, 1
        );

        // The overwrite IS applied (role deny), so SEND_MESSAGES is denied
        // This documents the expected behavior: caller must filter correctly
        assert!(perms & Permissions::SEND_MESSAGES == 0);
    }
}
