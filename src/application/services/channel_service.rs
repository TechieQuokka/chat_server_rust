//! Channel Service
//!
//! Handles channel management operations.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    Channel, ChannelRepository, ChannelType, MemberRepository,
    PermissionOverwrite, ServerRepository,
};
use crate::shared::snowflake::SnowflakeGenerator;

/// Channel service trait
#[async_trait]
pub trait ChannelService: Send + Sync {
    /// Create a new channel in a guild
    async fn create_channel(&self, guild_id: i64, actor_id: i64, request: CreateChannelDto) -> Result<ChannelDto, ChannelError>;

    /// Get channel by ID
    async fn get_channel(&self, channel_id: i64) -> Result<ChannelDto, ChannelError>;

    /// Update channel
    async fn update_channel(&self, channel_id: i64, actor_id: i64, update: UpdateChannelDto) -> Result<ChannelDto, ChannelError>;

    /// Delete channel
    async fn delete_channel(&self, channel_id: i64, actor_id: i64) -> Result<(), ChannelError>;

    /// Get channels for a guild
    async fn get_guild_channels(&self, guild_id: i64) -> Result<Vec<ChannelDto>, ChannelError>;

    /// Reorder channels
    async fn reorder_channels(&self, guild_id: i64, actor_id: i64, positions: Vec<(i64, i32)>) -> Result<(), ChannelError>;

    /// Set channel permission overwrites
    async fn set_permission_overwrites(
        &self,
        channel_id: i64,
        actor_id: i64,
        overwrites: Vec<PermissionOverwriteDto>,
    ) -> Result<(), ChannelError>;
}

/// Create channel request
#[derive(Debug, Clone)]
pub struct CreateChannelDto {
    pub name: String,
    pub channel_type: Option<String>,
    pub topic: Option<String>,
    pub parent_id: Option<i64>,
    pub position: Option<i32>,
    pub nsfw: Option<bool>,
}

/// Channel data transfer object
#[derive(Debug, Clone)]
pub struct ChannelDto {
    pub id: String,
    pub guild_id: Option<String>,
    pub name: String,
    pub channel_type: String,
    pub topic: Option<String>,
    pub position: i32,
    pub parent_id: Option<String>,
    pub nsfw: bool,
    pub rate_limit_per_user: i32,
    pub created_at: String,
}

impl From<Channel> for ChannelDto {
    fn from(channel: Channel) -> Self {
        Self {
            id: channel.id.to_string(),
            guild_id: channel.server_id.map(|id| id.to_string()),
            name: channel.name,
            channel_type: channel.channel_type.as_str().to_string(),
            topic: channel.topic,
            position: channel.position,
            parent_id: channel.parent_id.map(|id| id.to_string()),
            nsfw: channel.nsfw,
            rate_limit_per_user: channel.rate_limit_per_user,
            created_at: channel.created_at.to_rfc3339(),
        }
    }
}

/// Update channel request
#[derive(Debug, Clone, Default)]
pub struct UpdateChannelDto {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub position: Option<i32>,
    pub parent_id: Option<Option<i64>>,
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<i32>,
}

/// Permission overwrite DTO
#[derive(Debug, Clone)]
pub struct PermissionOverwriteDto {
    pub target_id: i64,
    pub target_type: String,
    pub allow: i64,
    pub deny: i64,
}

/// Channel service errors
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel not found")]
    NotFound,

    #[error("Guild not found")]
    GuildNotFound,

    #[error("Permission denied")]
    Forbidden,

    #[error("Invalid channel type")]
    InvalidChannelType,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// ChannelService implementation
pub struct ChannelServiceImpl<C, S, M>
where
    C: ChannelRepository,
    S: ServerRepository,
    M: MemberRepository,
{
    channel_repo: Arc<C>,
    server_repo: Arc<S>,
    member_repo: Arc<M>,
    id_generator: Arc<SnowflakeGenerator>,
}

impl<C, S, M> ChannelServiceImpl<C, S, M>
where
    C: ChannelRepository,
    S: ServerRepository,
    M: MemberRepository,
{
    pub fn new(
        channel_repo: Arc<C>,
        server_repo: Arc<S>,
        member_repo: Arc<M>,
        id_generator: Arc<SnowflakeGenerator>,
    ) -> Self {
        Self {
            channel_repo,
            server_repo,
            member_repo,
            id_generator,
        }
    }

    async fn check_guild_permission(&self, guild_id: i64, user_id: i64) -> Result<bool, ChannelError> {
        // Check if user is owner (simplified - full implementation would check manage_channels permission)
        let server = self
            .server_repo
            .find_by_id(guild_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?
            .ok_or(ChannelError::GuildNotFound)?;

        Ok(server.owner_id == user_id)
    }

    fn parse_channel_type(type_str: Option<&str>) -> ChannelType {
        match type_str {
            Some("voice") => ChannelType::Voice,
            Some("category") => ChannelType::Category,
            Some("dm") => ChannelType::Dm,
            Some("group_dm") => ChannelType::GroupDm,
            _ => ChannelType::Text,
        }
    }
}

#[async_trait]
impl<C, S, M> ChannelService for ChannelServiceImpl<C, S, M>
where
    C: ChannelRepository + 'static,
    S: ServerRepository + 'static,
    M: MemberRepository + 'static,
{
    async fn create_channel(&self, guild_id: i64, actor_id: i64, request: CreateChannelDto) -> Result<ChannelDto, ChannelError> {
        // Check permission
        if !self.check_guild_permission(guild_id, actor_id).await? {
            return Err(ChannelError::Forbidden);
        }

        let now = Utc::now();
        let channel_type = Self::parse_channel_type(request.channel_type.as_deref());

        let channel = Channel {
            id: self.id_generator.generate(),
            server_id: Some(guild_id),
            name: request.name,
            channel_type,
            topic: request.topic,
            position: request.position.unwrap_or(0),
            parent_id: request.parent_id,
            nsfw: request.nsfw.unwrap_or(false),
            rate_limit_per_user: 0,
            created_at: now,
            updated_at: now,
        };

        let created = self
            .channel_repo
            .create(&channel)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(ChannelDto::from(created))
    }

    async fn get_channel(&self, channel_id: i64) -> Result<ChannelDto, ChannelError> {
        let channel = self
            .channel_repo
            .find_by_id(channel_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?
            .ok_or(ChannelError::NotFound)?;

        Ok(ChannelDto::from(channel))
    }

    async fn update_channel(&self, channel_id: i64, actor_id: i64, update: UpdateChannelDto) -> Result<ChannelDto, ChannelError> {
        let mut channel = self
            .channel_repo
            .find_by_id(channel_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?
            .ok_or(ChannelError::NotFound)?;

        // Check permission
        if let Some(guild_id) = channel.server_id {
            if !self.check_guild_permission(guild_id, actor_id).await? {
                return Err(ChannelError::Forbidden);
            }
        }

        // Apply updates
        if let Some(name) = update.name {
            channel.name = name;
        }
        if let Some(topic) = update.topic {
            channel.topic = Some(topic);
        }
        if let Some(position) = update.position {
            channel.position = position;
        }
        if let Some(parent_id) = update.parent_id {
            channel.parent_id = parent_id;
        }
        if let Some(nsfw) = update.nsfw {
            channel.nsfw = nsfw;
        }
        if let Some(rate_limit) = update.rate_limit_per_user {
            channel.rate_limit_per_user = rate_limit;
        }

        let updated = self
            .channel_repo
            .update(&channel)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(ChannelDto::from(updated))
    }

    async fn delete_channel(&self, channel_id: i64, actor_id: i64) -> Result<(), ChannelError> {
        let channel = self
            .channel_repo
            .find_by_id(channel_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?
            .ok_or(ChannelError::NotFound)?;

        // Check permission
        if let Some(guild_id) = channel.server_id {
            if !self.check_guild_permission(guild_id, actor_id).await? {
                return Err(ChannelError::Forbidden);
            }
        }

        self.channel_repo
            .delete(channel_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_guild_channels(&self, guild_id: i64) -> Result<Vec<ChannelDto>, ChannelError> {
        let channels = self
            .channel_repo
            .find_by_server_id(guild_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(channels.into_iter().map(ChannelDto::from).collect())
    }

    async fn reorder_channels(&self, guild_id: i64, actor_id: i64, positions: Vec<(i64, i32)>) -> Result<(), ChannelError> {
        // Check permission
        if !self.check_guild_permission(guild_id, actor_id).await? {
            return Err(ChannelError::Forbidden);
        }

        self.channel_repo
            .update_positions(guild_id, positions)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn set_permission_overwrites(
        &self,
        channel_id: i64,
        actor_id: i64,
        overwrites: Vec<PermissionOverwriteDto>,
    ) -> Result<(), ChannelError> {
        let channel = self
            .channel_repo
            .find_by_id(channel_id)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?
            .ok_or(ChannelError::NotFound)?;

        // Check permission
        if let Some(guild_id) = channel.server_id {
            if !self.check_guild_permission(guild_id, actor_id).await? {
                return Err(ChannelError::Forbidden);
            }
        }

        let domain_overwrites: Vec<PermissionOverwrite> = overwrites
            .into_iter()
            .map(|o| PermissionOverwrite {
                channel_id,
                target_id: o.target_id,
                target_type: o.target_type,
                allow: o.allow,
                deny: o.deny,
            })
            .collect();

        self.channel_repo
            .set_permission_overwrites(channel_id, domain_overwrites)
            .await
            .map_err(|e| ChannelError::Internal(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here
}
