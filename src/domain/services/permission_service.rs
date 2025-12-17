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

    #[test]
    fn test_owner_has_all_permissions() {
        let member = create_test_member(1, 100, vec![]);
        let roles = vec![];

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
}
