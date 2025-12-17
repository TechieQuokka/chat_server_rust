//! Role Service
//!
//! Handles role management operations including CRUD, reordering,
//! and member role assignments.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{MemberRepository, Role, RoleRepository, ServerRepository};
use crate::domain::value_objects::Permissions;
use crate::shared::snowflake::SnowflakeGenerator;

/// Role service trait defining all role management operations.
#[async_trait]
pub trait RoleService: Send + Sync {
    /// Create a new role in a server.
    async fn create_role(
        &self,
        server_id: i64,
        actor_id: i64,
        request: CreateRoleDto,
    ) -> Result<RoleDto, RoleError>;

    /// Get a role by its ID.
    async fn get_role(&self, role_id: i64) -> Result<RoleDto, RoleError>;

    /// Get all roles for a server.
    async fn get_roles_by_server(&self, server_id: i64) -> Result<Vec<RoleDto>, RoleError>;

    /// Update a role.
    async fn update_role(
        &self,
        role_id: i64,
        actor_id: i64,
        update: UpdateRoleDto,
    ) -> Result<RoleDto, RoleError>;

    /// Delete a role.
    async fn delete_role(&self, role_id: i64, actor_id: i64) -> Result<(), RoleError>;

    /// Reorder roles within a server.
    async fn reorder_roles(
        &self,
        server_id: i64,
        actor_id: i64,
        positions: Vec<RolePositionDto>,
    ) -> Result<(), RoleError>;

    /// Assign a role to a member.
    async fn assign_role_to_member(
        &self,
        server_id: i64,
        user_id: i64,
        role_id: i64,
        actor_id: i64,
    ) -> Result<(), RoleError>;

    /// Remove a role from a member.
    async fn remove_role_from_member(
        &self,
        server_id: i64,
        user_id: i64,
        role_id: i64,
        actor_id: i64,
    ) -> Result<(), RoleError>;

    /// Get all roles assigned to a member.
    async fn get_member_roles(
        &self,
        server_id: i64,
        user_id: i64,
    ) -> Result<Vec<RoleDto>, RoleError>;
}

// =============================================================================
// Data Transfer Objects
// =============================================================================

/// Create role request DTO.
#[derive(Debug, Clone)]
pub struct CreateRoleDto {
    /// Role name (1-100 characters).
    pub name: String,
    /// Permission bitfield.
    pub permissions: Option<i64>,
    /// Role color as RGB integer.
    pub color: Option<i32>,
    /// Whether to display role members separately in the member list.
    pub hoist: Option<bool>,
    /// Whether this role can be mentioned by everyone.
    pub mentionable: Option<bool>,
}

/// Role data transfer object.
#[derive(Debug, Clone)]
pub struct RoleDto {
    /// Role ID (snowflake).
    pub id: String,
    /// Server ID this role belongs to.
    pub server_id: String,
    /// Role name.
    pub name: String,
    /// Permission bitfield as string (for JavaScript compatibility).
    pub permissions: String,
    /// Position in the role hierarchy.
    pub position: i32,
    /// Role color as RGB integer.
    pub color: Option<i32>,
    /// Whether members are displayed separately.
    pub hoist: bool,
    /// Whether this role can be mentioned.
    pub mentionable: bool,
    /// Whether this is a managed role (bot roles, integrations).
    pub managed: bool,
    /// Role creation timestamp.
    pub created_at: String,
}

impl From<Role> for RoleDto {
    fn from(role: Role) -> Self {
        Self {
            id: role.id.to_string(),
            server_id: role.server_id.to_string(),
            name: role.name,
            permissions: role.permissions.to_string(),
            position: role.position,
            color: role.color,
            hoist: role.hoist,
            mentionable: role.mentionable,
            managed: false, // We don't have managed roles yet
            created_at: role.created_at.to_rfc3339(),
        }
    }
}

/// Update role request DTO.
#[derive(Debug, Clone, Default)]
pub struct UpdateRoleDto {
    /// New role name.
    pub name: Option<String>,
    /// New permission bitfield.
    pub permissions: Option<i64>,
    /// New role color.
    pub color: Option<Option<i32>>,
    /// New hoist setting.
    pub hoist: Option<bool>,
    /// New mentionable setting.
    pub mentionable: Option<bool>,
}

/// Role position DTO for reordering.
#[derive(Debug, Clone)]
pub struct RolePositionDto {
    /// Role ID.
    pub id: i64,
    /// New position.
    pub position: i32,
}

// =============================================================================
// Error Types
// =============================================================================

/// Role service errors.
#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("Role not found")]
    NotFound,

    #[error("Server not found")]
    ServerNotFound,

    #[error("Member not found")]
    MemberNotFound,

    #[error("Permission denied")]
    Forbidden,

    #[error("Cannot modify @everyone role")]
    CannotModifyEveryoneRole,

    #[error("Cannot delete @everyone role")]
    CannotDeleteEveryoneRole,

    #[error("Cannot assign @everyone role")]
    CannotAssignEveryoneRole,

    #[error("Role hierarchy violation: cannot modify role higher than your highest role")]
    HierarchyViolation,

    #[error("Invalid role name: {0}")]
    InvalidName(String),

    #[error("Invalid permissions value")]
    InvalidPermissions,

    #[error("Internal error: {0}")]
    Internal(String),
}

// =============================================================================
// Service Implementation
// =============================================================================

/// RoleService implementation with PostgreSQL repositories.
pub struct RoleServiceImpl<R, S, M>
where
    R: RoleRepository,
    S: ServerRepository,
    M: MemberRepository,
{
    role_repo: Arc<R>,
    server_repo: Arc<S>,
    member_repo: Arc<M>,
    id_generator: Arc<SnowflakeGenerator>,
}

impl<R, S, M> RoleServiceImpl<R, S, M>
where
    R: RoleRepository,
    S: ServerRepository,
    M: MemberRepository,
{
    /// Create a new RoleServiceImpl.
    pub fn new(
        role_repo: Arc<R>,
        server_repo: Arc<S>,
        member_repo: Arc<M>,
        id_generator: Arc<SnowflakeGenerator>,
    ) -> Self {
        Self {
            role_repo,
            server_repo,
            member_repo,
            id_generator,
        }
    }

    /// Check if the user is the server owner.
    async fn is_owner(&self, server_id: i64, user_id: i64) -> Result<bool, RoleError> {
        let server = self
            .server_repo
            .find_by_id(server_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::ServerNotFound)?;

        Ok(server.owner_id == user_id)
    }

    /// Check if the actor has permission to manage roles.
    ///
    /// Returns true if:
    /// - Actor is the server owner, OR
    /// - Actor has MANAGE_ROLES or ADMINISTRATOR permission
    async fn can_manage_roles(&self, server_id: i64, actor_id: i64) -> Result<bool, RoleError> {
        // Server owner always has permission
        if self.is_owner(server_id, actor_id).await? {
            return Ok(true);
        }

        // Check if actor is a member
        let is_member = self
            .member_repo
            .is_member(server_id, actor_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        if !is_member {
            return Err(RoleError::Forbidden);
        }

        // Get actor's roles and compute permissions
        let role_ids = self
            .member_repo
            .get_roles(server_id, actor_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        let mut permissions = 0_i64;

        // Get @everyone role permissions
        if let Some(everyone) = self
            .role_repo
            .find_everyone_role(server_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
        {
            permissions |= everyone.permissions;
        }

        // Add permissions from assigned roles
        for role_id in role_ids {
            if let Some(role) = self
                .role_repo
                .find_by_id(role_id)
                .await
                .map_err(|e| RoleError::Internal(e.to_string()))?
            {
                permissions |= role.permissions;
            }
        }

        let perms = Permissions::new(permissions);
        Ok(perms.has(Permissions::MANAGE_ROLES) || perms.is_admin())
    }

    /// Get the highest role position for the actor.
    async fn get_actor_highest_role_position(
        &self,
        server_id: i64,
        actor_id: i64,
    ) -> Result<i32, RoleError> {
        // Owner has the highest position
        if self.is_owner(server_id, actor_id).await? {
            return Ok(i32::MAX);
        }

        let role_ids = self
            .member_repo
            .get_roles(server_id, actor_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        let mut highest = 0;

        for role_id in role_ids {
            if let Some(role) = self
                .role_repo
                .find_by_id(role_id)
                .await
                .map_err(|e| RoleError::Internal(e.to_string()))?
            {
                if role.position > highest {
                    highest = role.position;
                }
            }
        }

        Ok(highest)
    }

    /// Check if the target role is within the actor's hierarchy.
    async fn check_hierarchy(
        &self,
        server_id: i64,
        actor_id: i64,
        target_role_position: i32,
    ) -> Result<(), RoleError> {
        let actor_highest = self
            .get_actor_highest_role_position(server_id, actor_id)
            .await?;

        if target_role_position >= actor_highest {
            return Err(RoleError::HierarchyViolation);
        }

        Ok(())
    }

    /// Check if a role is the @everyone role.
    fn is_everyone_role(role: &Role) -> bool {
        // @everyone role has the same ID as the server, or position 0 and name "@everyone"
        role.id == role.server_id || (role.position == 0 && role.name == "@everyone")
    }

    /// Validate role name.
    fn validate_name(name: &str) -> Result<(), RoleError> {
        if name.is_empty() {
            return Err(RoleError::InvalidName("Name cannot be empty".to_string()));
        }
        if name.len() > 100 {
            return Err(RoleError::InvalidName(
                "Name must be at most 100 characters".to_string(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl<R, S, M> RoleService for RoleServiceImpl<R, S, M>
where
    R: RoleRepository + 'static,
    S: ServerRepository + 'static,
    M: MemberRepository + 'static,
{
    async fn create_role(
        &self,
        server_id: i64,
        actor_id: i64,
        request: CreateRoleDto,
    ) -> Result<RoleDto, RoleError> {
        // Check permission
        if !self.can_manage_roles(server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        // Validate name
        Self::validate_name(&request.name)?;

        // Get the next position (one above the highest)
        let max_position = self
            .role_repo
            .get_max_position(server_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        let now = Utc::now();

        let role = Role {
            id: self.id_generator.generate(),
            server_id,
            name: request.name,
            permissions: request.permissions.unwrap_or(0),
            position: max_position + 1,
            color: request.color,
            hoist: request.hoist.unwrap_or(false),
            mentionable: request.mentionable.unwrap_or(false),
            created_at: now,
            updated_at: now,
        };

        let created = self
            .role_repo
            .create(&role)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(RoleDto::from(created))
    }

    async fn get_role(&self, role_id: i64) -> Result<RoleDto, RoleError> {
        let role = self
            .role_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::NotFound)?;

        Ok(RoleDto::from(role))
    }

    async fn get_roles_by_server(&self, server_id: i64) -> Result<Vec<RoleDto>, RoleError> {
        // Verify server exists
        self.server_repo
            .find_by_id(server_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::ServerNotFound)?;

        let roles = self
            .role_repo
            .find_by_server_id(server_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(roles.into_iter().map(RoleDto::from).collect())
    }

    async fn update_role(
        &self,
        role_id: i64,
        actor_id: i64,
        update: UpdateRoleDto,
    ) -> Result<RoleDto, RoleError> {
        let mut role = self
            .role_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::NotFound)?;

        // Check permission
        if !self.can_manage_roles(role.server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        // Cannot fully modify @everyone role (only permissions can be changed)
        let is_everyone = Self::is_everyone_role(&role);

        // Check hierarchy - cannot modify roles at or above actor's highest role
        if !is_everyone {
            self.check_hierarchy(role.server_id, actor_id, role.position)
                .await?;
        }

        // Apply updates
        if let Some(name) = update.name {
            if is_everyone {
                // @everyone role name cannot be changed
                return Err(RoleError::CannotModifyEveryoneRole);
            }
            Self::validate_name(&name)?;
            role.name = name;
        }

        if let Some(permissions) = update.permissions {
            role.permissions = permissions;
        }

        if let Some(color) = update.color {
            if is_everyone {
                return Err(RoleError::CannotModifyEveryoneRole);
            }
            role.color = color;
        }

        if let Some(hoist) = update.hoist {
            if is_everyone {
                return Err(RoleError::CannotModifyEveryoneRole);
            }
            role.hoist = hoist;
        }

        if let Some(mentionable) = update.mentionable {
            if is_everyone {
                return Err(RoleError::CannotModifyEveryoneRole);
            }
            role.mentionable = mentionable;
        }

        role.updated_at = Utc::now();

        let updated = self
            .role_repo
            .update(&role)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(RoleDto::from(updated))
    }

    async fn delete_role(&self, role_id: i64, actor_id: i64) -> Result<(), RoleError> {
        let role = self
            .role_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::NotFound)?;

        // Cannot delete @everyone role
        if Self::is_everyone_role(&role) {
            return Err(RoleError::CannotDeleteEveryoneRole);
        }

        // Check permission
        if !self.can_manage_roles(role.server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        // Check hierarchy
        self.check_hierarchy(role.server_id, actor_id, role.position)
            .await?;

        self.role_repo
            .delete(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn reorder_roles(
        &self,
        server_id: i64,
        actor_id: i64,
        positions: Vec<RolePositionDto>,
    ) -> Result<(), RoleError> {
        // Check permission
        if !self.can_manage_roles(server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        let actor_highest = self
            .get_actor_highest_role_position(server_id, actor_id)
            .await?;

        // Validate all positions and check hierarchy
        for pos in &positions {
            let role = self
                .role_repo
                .find_by_id(pos.id)
                .await
                .map_err(|e| RoleError::Internal(e.to_string()))?
                .ok_or(RoleError::NotFound)?;

            // Verify role belongs to the server
            if role.server_id != server_id {
                return Err(RoleError::NotFound);
            }

            // Cannot reorder @everyone role
            if Self::is_everyone_role(&role) {
                return Err(RoleError::CannotModifyEveryoneRole);
            }

            // Check hierarchy - cannot move roles at or above actor's position
            if role.position >= actor_highest {
                return Err(RoleError::HierarchyViolation);
            }

            // Cannot move role to position at or above actor's position
            if pos.position >= actor_highest {
                return Err(RoleError::HierarchyViolation);
            }
        }

        let position_updates: Vec<(i64, i32)> =
            positions.into_iter().map(|p| (p.id, p.position)).collect();

        self.role_repo
            .update_positions(server_id, position_updates)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn assign_role_to_member(
        &self,
        server_id: i64,
        user_id: i64,
        role_id: i64,
        actor_id: i64,
    ) -> Result<(), RoleError> {
        // Check permission
        if !self.can_manage_roles(server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        // Verify the role exists and belongs to the server
        let role = self
            .role_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::NotFound)?;

        if role.server_id != server_id {
            return Err(RoleError::NotFound);
        }

        // Cannot assign @everyone role (everyone has it implicitly)
        if Self::is_everyone_role(&role) {
            return Err(RoleError::CannotAssignEveryoneRole);
        }

        // Check hierarchy
        self.check_hierarchy(server_id, actor_id, role.position)
            .await?;

        // Verify the user is a member of the server
        let is_member = self
            .member_repo
            .is_member(server_id, user_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        if !is_member {
            return Err(RoleError::MemberNotFound);
        }

        // Add the role to the member
        self.member_repo
            .add_role(server_id, user_id, role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn remove_role_from_member(
        &self,
        server_id: i64,
        user_id: i64,
        role_id: i64,
        actor_id: i64,
    ) -> Result<(), RoleError> {
        // Check permission
        if !self.can_manage_roles(server_id, actor_id).await? {
            return Err(RoleError::Forbidden);
        }

        // Verify the role exists and belongs to the server
        let role = self
            .role_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?
            .ok_or(RoleError::NotFound)?;

        if role.server_id != server_id {
            return Err(RoleError::NotFound);
        }

        // Cannot remove @everyone role
        if Self::is_everyone_role(&role) {
            return Err(RoleError::CannotAssignEveryoneRole);
        }

        // Check hierarchy
        self.check_hierarchy(server_id, actor_id, role.position)
            .await?;

        // Verify the user is a member of the server
        let is_member = self
            .member_repo
            .is_member(server_id, user_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        if !is_member {
            return Err(RoleError::MemberNotFound);
        }

        // Remove the role from the member
        self.member_repo
            .remove_role(server_id, user_id, role_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_member_roles(
        &self,
        server_id: i64,
        user_id: i64,
    ) -> Result<Vec<RoleDto>, RoleError> {
        // Verify the user is a member
        let is_member = self
            .member_repo
            .is_member(server_id, user_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        if !is_member {
            return Err(RoleError::MemberNotFound);
        }

        // Get role IDs for the member
        let role_ids = self
            .member_repo
            .get_roles(server_id, user_id)
            .await
            .map_err(|e| RoleError::Internal(e.to_string()))?;

        // Fetch full role objects
        let mut roles = Vec::with_capacity(role_ids.len());

        for role_id in role_ids {
            if let Some(role) = self
                .role_repo
                .find_by_id(role_id)
                .await
                .map_err(|e| RoleError::Internal(e.to_string()))?
            {
                roles.push(RoleDto::from(role));
            }
        }

        // Sort by position descending
        roles.sort_by(|a, b| b.position.cmp(&a.position));

        Ok(roles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_dto_from_role() {
        let now = Utc::now();
        let role = Role {
            id: 123456789,
            server_id: 987654321,
            name: "Test Role".to_string(),
            permissions: 104324673,
            position: 5,
            color: Some(0xFF5733),
            hoist: true,
            mentionable: false,
            created_at: now,
            updated_at: now,
        };

        let dto = RoleDto::from(role);

        assert_eq!(dto.id, "123456789");
        assert_eq!(dto.server_id, "987654321");
        assert_eq!(dto.name, "Test Role");
        assert_eq!(dto.permissions, "104324673");
        assert_eq!(dto.position, 5);
        assert_eq!(dto.color, Some(0xFF5733));
        assert!(dto.hoist);
        assert!(!dto.mentionable);
        assert!(!dto.managed);
    }

    #[test]
    fn test_validate_name_empty() {
        let result = RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::validate_name("");

        assert!(matches!(result, Err(RoleError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(101);
        let result = RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::validate_name(&long_name);

        assert!(matches!(result, Err(RoleError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_valid() {
        let result = RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::validate_name("Moderator");

        assert!(result.is_ok());
    }

    #[test]
    fn test_is_everyone_role_by_id() {
        let role = Role {
            id: 100,
            server_id: 100, // Same as id
            name: "anything".to_string(),
            permissions: 0,
            position: 5,
            color: None,
            hoist: false,
            mentionable: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::is_everyone_role(&role));
    }

    #[test]
    fn test_is_everyone_role_by_name_and_position() {
        let role = Role {
            id: 200,
            server_id: 100,
            name: "@everyone".to_string(),
            permissions: 0,
            position: 0,
            color: None,
            hoist: false,
            mentionable: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::is_everyone_role(&role));
    }

    #[test]
    fn test_is_not_everyone_role() {
        let role = Role {
            id: 200,
            server_id: 100,
            name: "Admin".to_string(),
            permissions: 0,
            position: 5,
            color: None,
            hoist: false,
            mentionable: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!RoleServiceImpl::<
            crate::infrastructure::repositories::PgRoleRepository,
            crate::infrastructure::repositories::PgServerRepository,
            crate::infrastructure::repositories::PgMemberRepository,
        >::is_everyone_role(&role));
    }
}
