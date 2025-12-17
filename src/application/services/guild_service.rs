//! Guild/Server Service
//!
//! Handles guild/server management operations.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    Channel, ChannelRepository, ChannelType, Member, MemberRepository,
    Role, RoleRepository, Server, ServerRepository,
};
use crate::domain::value_objects::Permissions;
use crate::shared::snowflake::SnowflakeGenerator;

/// Guild service trait
#[async_trait]
pub trait GuildService: Send + Sync {
    /// Create a new guild
    async fn create_guild(&self, owner_id: i64, request: CreateGuildDto) -> Result<GuildDto, GuildError>;

    /// Get guild by ID
    async fn get_guild(&self, guild_id: i64) -> Result<GuildDto, GuildError>;

    /// Update guild settings
    async fn update_guild(&self, guild_id: i64, actor_id: i64, update: UpdateGuildDto) -> Result<GuildDto, GuildError>;

    /// Delete guild
    async fn delete_guild(&self, guild_id: i64, actor_id: i64) -> Result<(), GuildError>;

    /// Get guild members
    async fn get_members(&self, guild_id: i64, after: Option<i64>, limit: i32) -> Result<Vec<MemberDto>, GuildError>;

    /// Join a guild (via invite)
    async fn join_guild(&self, guild_id: i64, user_id: i64) -> Result<MemberDto, GuildError>;

    /// Leave a guild
    async fn leave_guild(&self, guild_id: i64, user_id: i64) -> Result<(), GuildError>;

    /// Kick a member
    async fn kick_member(&self, guild_id: i64, actor_id: i64, target_id: i64) -> Result<(), GuildError>;

    /// Transfer ownership
    async fn transfer_ownership(&self, guild_id: i64, owner_id: i64, new_owner_id: i64) -> Result<(), GuildError>;
}

/// Create guild request
#[derive(Debug, Clone)]
pub struct CreateGuildDto {
    pub name: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// Guild data transfer object
#[derive(Debug, Clone)]
pub struct GuildDto {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
    pub member_count: i64,
    pub created_at: String,
}

impl GuildDto {
    pub fn from_server(server: Server, member_count: i64) -> Self {
        Self {
            id: server.id.to_string(),
            name: server.name,
            owner_id: server.owner_id.to_string(),
            icon_url: server.icon_url,
            description: server.description,
            member_count,
            created_at: server.created_at.to_rfc3339(),
        }
    }
}

/// Update guild request
#[derive(Debug, Clone, Default)]
pub struct UpdateGuildDto {
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// Member data transfer object
#[derive(Debug, Clone)]
pub struct MemberDto {
    pub user_id: String,
    pub server_id: String,
    pub nickname: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
}

impl From<Member> for MemberDto {
    fn from(member: Member) -> Self {
        Self {
            user_id: member.user_id.to_string(),
            server_id: member.server_id.to_string(),
            nickname: member.nickname,
            roles: member.roles.iter().map(|r| r.to_string()).collect(),
            joined_at: member.joined_at.to_rfc3339(),
        }
    }
}

/// Guild service errors
#[derive(Debug, thiserror::Error)]
pub enum GuildError {
    #[error("Guild not found")]
    NotFound,

    #[error("Permission denied")]
    Forbidden,

    #[error("Already a member")]
    AlreadyMember,

    #[error("Cannot leave as owner")]
    CannotLeaveAsOwner,

    #[error("Member not found")]
    MemberNotFound,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// GuildService implementation
pub struct GuildServiceImpl<S, C, M, R>
where
    S: ServerRepository,
    C: ChannelRepository,
    M: MemberRepository,
    R: RoleRepository,
{
    server_repo: Arc<S>,
    channel_repo: Arc<C>,
    member_repo: Arc<M>,
    role_repo: Arc<R>,
    id_generator: Arc<SnowflakeGenerator>,
}

impl<S, C, M, R> GuildServiceImpl<S, C, M, R>
where
    S: ServerRepository,
    C: ChannelRepository,
    M: MemberRepository,
    R: RoleRepository,
{
    pub fn new(
        server_repo: Arc<S>,
        channel_repo: Arc<C>,
        member_repo: Arc<M>,
        role_repo: Arc<R>,
        id_generator: Arc<SnowflakeGenerator>,
    ) -> Self {
        Self {
            server_repo,
            channel_repo,
            member_repo,
            role_repo,
            id_generator,
        }
    }

    async fn is_owner(&self, guild_id: i64, user_id: i64) -> Result<bool, GuildError> {
        let server = self
            .server_repo
            .find_by_id(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?
            .ok_or(GuildError::NotFound)?;

        Ok(server.owner_id == user_id)
    }
}

#[async_trait]
impl<S, C, M, R> GuildService for GuildServiceImpl<S, C, M, R>
where
    S: ServerRepository + 'static,
    C: ChannelRepository + 'static,
    M: MemberRepository + 'static,
    R: RoleRepository + 'static,
{
    async fn create_guild(&self, owner_id: i64, request: CreateGuildDto) -> Result<GuildDto, GuildError> {
        let now = Utc::now();
        let server_id = self.id_generator.generate();

        // Create server
        let server = Server {
            id: server_id,
            name: request.name,
            owner_id,
            icon_url: request.icon_url,
            description: request.description,
            created_at: now,
            updated_at: now,
        };

        let created_server = self
            .server_repo
            .create(&server)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        // Create @everyone role (same ID as server)
        let everyone_role = Role {
            id: server_id,
            server_id,
            name: "@everyone".to_string(),
            color: None,
            hoist: false,
            position: 0,
            permissions: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
            mentionable: false,
            created_at: now,
            updated_at: now,
        };

        self.role_repo
            .create(&everyone_role)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        // Create default #general text channel
        let general_channel = Channel {
            id: self.id_generator.generate(),
            server_id: Some(server_id),
            name: "general".to_string(),
            channel_type: ChannelType::Text,
            topic: Some("General discussion".to_string()),
            position: 0,
            parent_id: None,
            nsfw: false,
            rate_limit_per_user: 0,
            created_at: now,
            updated_at: now,
        };

        self.channel_repo
            .create(&general_channel)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(GuildDto::from_server(created_server, 1))
    }

    async fn get_guild(&self, guild_id: i64) -> Result<GuildDto, GuildError> {
        let server = self
            .server_repo
            .find_by_id(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?
            .ok_or(GuildError::NotFound)?;

        let member_count = self
            .member_repo
            .count_by_server(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(GuildDto::from_server(server, member_count))
    }

    async fn update_guild(&self, guild_id: i64, actor_id: i64, update: UpdateGuildDto) -> Result<GuildDto, GuildError> {
        // Check if actor is owner
        if !self.is_owner(guild_id, actor_id).await? {
            return Err(GuildError::Forbidden);
        }

        let mut server = self
            .server_repo
            .find_by_id(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?
            .ok_or(GuildError::NotFound)?;

        // Apply updates
        if let Some(name) = update.name {
            server.name = name;
        }
        if let Some(icon_url) = update.icon_url {
            server.icon_url = Some(icon_url);
        }
        if let Some(description) = update.description {
            server.description = Some(description);
        }

        let updated = self
            .server_repo
            .update(&server)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        let member_count = self
            .member_repo
            .count_by_server(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(GuildDto::from_server(updated, member_count))
    }

    async fn delete_guild(&self, guild_id: i64, actor_id: i64) -> Result<(), GuildError> {
        // Only owner can delete
        if !self.is_owner(guild_id, actor_id).await? {
            return Err(GuildError::Forbidden);
        }

        self.server_repo
            .delete(guild_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_members(&self, guild_id: i64, after: Option<i64>, limit: i32) -> Result<Vec<MemberDto>, GuildError> {
        let members = self
            .member_repo
            .find_by_server_id(guild_id, after, limit)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(members.into_iter().map(MemberDto::from).collect())
    }

    async fn join_guild(&self, guild_id: i64, user_id: i64) -> Result<MemberDto, GuildError> {
        // Check if already a member
        let is_member = self
            .member_repo
            .is_member(guild_id, user_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        if is_member {
            return Err(GuildError::AlreadyMember);
        }

        // Create member
        let member = Member {
            server_id: guild_id,
            user_id,
            nickname: None,
            joined_at: Utc::now(),
            roles: vec![],
        };

        let created = self
            .member_repo
            .create(&member)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(MemberDto::from(created))
    }

    async fn leave_guild(&self, guild_id: i64, user_id: i64) -> Result<(), GuildError> {
        // Owner cannot leave
        if self.is_owner(guild_id, user_id).await? {
            return Err(GuildError::CannotLeaveAsOwner);
        }

        self.member_repo
            .delete(guild_id, user_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn kick_member(&self, guild_id: i64, actor_id: i64, target_id: i64) -> Result<(), GuildError> {
        // Only owner can kick (simplified - full implementation would check permissions)
        if !self.is_owner(guild_id, actor_id).await? {
            return Err(GuildError::Forbidden);
        }

        // Cannot kick owner
        if self.is_owner(guild_id, target_id).await? {
            return Err(GuildError::Forbidden);
        }

        self.member_repo
            .delete(guild_id, target_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn transfer_ownership(&self, guild_id: i64, owner_id: i64, new_owner_id: i64) -> Result<(), GuildError> {
        // Verify current owner
        if !self.is_owner(guild_id, owner_id).await? {
            return Err(GuildError::Forbidden);
        }

        // Verify new owner is a member
        let is_member = self
            .member_repo
            .is_member(guild_id, new_owner_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        if !is_member {
            return Err(GuildError::MemberNotFound);
        }

        self.server_repo
            .transfer_ownership(guild_id, new_owner_id)
            .await
            .map_err(|e| GuildError::Internal(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here
}
