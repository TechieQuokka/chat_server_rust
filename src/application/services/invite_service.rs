//! Invite Service
//!
//! Handles server invite operations including creation, validation, and usage.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::domain::{Invite, InviteRepository, MemberRepository};
use crate::infrastructure::repositories::PgInviteRepository;
use crate::application::services::{GuildService, GuildError};

/// Invite service trait defining invite operations.
#[async_trait]
pub trait InviteService: Send + Sync {
    /// Create a new invite for a server channel.
    async fn create_invite(
        &self,
        request: CreateInviteDto,
        inviter_id: i64,
    ) -> Result<InviteDto, InviteError>;

    /// Get an invite by its code.
    async fn get_invite(&self, code: &str) -> Result<InviteDto, InviteError>;

    /// Get invite preview (server info without full details).
    async fn get_invite_preview(&self, code: &str) -> Result<InvitePreviewDto, InviteError>;

    /// Get all invites for a server.
    async fn get_server_invites(&self, server_id: i64) -> Result<Vec<InviteDto>, InviteError>;

    /// Use an invite to join a server.
    async fn use_invite(&self, code: &str, user_id: i64) -> Result<UseInviteResultDto, InviteError>;

    /// Delete an invite by code.
    async fn delete_invite(&self, code: &str, actor_id: i64) -> Result<(), InviteError>;

    /// Validate an invite (check if still valid).
    async fn validate_invite(&self, code: &str) -> Result<InviteValidationDto, InviteError>;

    /// Get all invites created by a user.
    async fn get_user_invites(&self, user_id: i64) -> Result<Vec<InviteDto>, InviteError>;

    /// Clean up expired invites (maintenance task).
    async fn cleanup_expired(&self) -> Result<u64, InviteError>;
}

/// Request DTO for creating an invite.
#[derive(Debug, Clone)]
pub struct CreateInviteDto {
    /// Server ID to create invite for.
    pub server_id: i64,
    /// Channel ID the invite leads to.
    pub channel_id: i64,
    /// Maximum number of uses (0 = unlimited).
    pub max_uses: Option<i32>,
    /// Seconds until expiration (0 = never expires).
    /// Common values: 1800 (30 min), 3600 (1 hour), 86400 (24 hours), 604800 (7 days).
    pub max_age: Option<i32>,
    /// Whether members gain temporary membership.
    pub temporary: Option<bool>,
}

/// Invite data transfer object.
#[derive(Debug, Clone)]
pub struct InviteDto {
    /// Unique invite code.
    pub code: String,
    /// Server ID this invite is for.
    pub server_id: String,
    /// Channel ID the invite leads to.
    pub channel_id: String,
    /// User ID who created the invite.
    pub inviter_id: Option<String>,
    /// Maximum uses (0 = unlimited).
    pub max_uses: i32,
    /// Current use count.
    pub uses: i32,
    /// Max age in seconds (0 = never).
    pub max_age: i32,
    /// Whether membership is temporary.
    pub temporary: bool,
    /// Expiration timestamp (if any).
    pub expires_at: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Whether invite is currently valid.
    pub is_valid: bool,
}

impl InviteDto {
    /// Create DTO from domain Invite entity.
    pub fn from_invite(invite: Invite) -> Self {
        let is_valid = invite.is_valid();
        Self {
            code: invite.code,
            server_id: invite.server_id.to_string(),
            channel_id: invite.channel_id.to_string(),
            inviter_id: invite.inviter_id.map(|id| id.to_string()),
            max_uses: invite.max_uses,
            uses: invite.uses,
            max_age: invite.max_age,
            temporary: invite.temporary,
            expires_at: invite.expires_at.map(|dt| dt.to_rfc3339()),
            created_at: invite.created_at.to_rfc3339(),
            is_valid,
        }
    }
}

/// Invite preview DTO (used before joining).
#[derive(Debug, Clone)]
pub struct InvitePreviewDto {
    /// Invite code.
    pub code: String,
    /// Server ID.
    pub server_id: String,
    /// Server name.
    pub server_name: String,
    /// Server icon URL.
    pub server_icon: Option<String>,
    /// Channel ID the invite leads to.
    pub channel_id: String,
    /// Channel name.
    pub channel_name: String,
    /// Inviter user ID.
    pub inviter_id: Option<String>,
    /// Inviter username.
    pub inviter_name: Option<String>,
    /// Current member count.
    pub member_count: i64,
    /// Whether invite is valid.
    pub is_valid: bool,
}

/// Result of using an invite.
#[derive(Debug, Clone)]
pub struct UseInviteResultDto {
    /// Server ID joined.
    pub server_id: String,
    /// Whether user was already a member.
    pub already_member: bool,
}

/// Invite validation result.
#[derive(Debug, Clone)]
pub struct InviteValidationDto {
    /// Invite code.
    pub code: String,
    /// Whether invite is valid.
    pub is_valid: bool,
    /// Reason if invalid.
    pub invalid_reason: Option<String>,
    /// Remaining uses (None if unlimited).
    pub remaining_uses: Option<i32>,
    /// Time until expiration in seconds (None if never).
    pub expires_in: Option<i64>,
}

/// Invite service errors.
#[derive(Debug, thiserror::Error)]
pub enum InviteError {
    #[error("Invite not found")]
    NotFound,

    #[error("Invite has expired")]
    Expired,

    #[error("Invite has reached maximum uses")]
    MaxUsesReached,

    #[error("Invalid invite code")]
    InvalidCode,

    #[error("Permission denied")]
    Forbidden,

    #[error("Server not found")]
    ServerNotFound,

    #[error("Channel not found")]
    ChannelNotFound,

    #[error("Already a member of this server")]
    AlreadyMember,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<GuildError> for InviteError {
    fn from(err: GuildError) -> Self {
        match err {
            GuildError::NotFound => InviteError::ServerNotFound,
            GuildError::Forbidden => InviteError::Forbidden,
            GuildError::AlreadyMember => InviteError::AlreadyMember,
            GuildError::Internal(msg) => InviteError::Internal(msg),
            _ => InviteError::Internal(err.to_string()),
        }
    }
}

/// Invite service implementation.
pub struct InviteServiceImpl<I, G, M>
where
    I: InviteRepository,
    G: GuildService,
    M: MemberRepository,
{
    invite_repo: Arc<I>,
    guild_service: Arc<G>,
    member_repo: Arc<M>,
}

impl<I, G, M> InviteServiceImpl<I, G, M>
where
    I: InviteRepository,
    G: GuildService,
    M: MemberRepository,
{
    /// Create a new InviteServiceImpl.
    pub fn new(
        invite_repo: Arc<I>,
        guild_service: Arc<G>,
        member_repo: Arc<M>,
    ) -> Self {
        Self {
            invite_repo,
            guild_service,
            member_repo,
        }
    }

    /// Generate a unique invite code (8 alphanumeric characters).
    fn generate_unique_code() -> String {
        Invite::generate_code()
    }

    /// Check if actor has permission to manage invites for a server.
    async fn can_manage_invites(&self, server_id: i64, actor_id: i64) -> Result<bool, InviteError> {
        // Check if actor is a member
        let is_member = self
            .member_repo
            .is_member(server_id, actor_id)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        // For now, any member can create/manage invites
        // A more complete implementation would check permissions
        Ok(is_member)
    }
}

#[async_trait]
impl<I, G, M> InviteService for InviteServiceImpl<I, G, M>
where
    I: InviteRepository + 'static,
    G: GuildService + 'static,
    M: MemberRepository + 'static,
{
    async fn create_invite(
        &self,
        request: CreateInviteDto,
        inviter_id: i64,
    ) -> Result<InviteDto, InviteError> {
        // Verify server exists
        self.guild_service
            .get_guild(request.server_id)
            .await
            .map_err(|_| InviteError::ServerNotFound)?;

        // Verify actor can create invites
        if !self.can_manage_invites(request.server_id, inviter_id).await? {
            return Err(InviteError::Forbidden);
        }

        // Generate unique code with collision retry
        let mut code = Self::generate_unique_code();
        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 5;

        while self.invite_repo.code_exists(&code).await.map_err(|e| InviteError::Internal(e.to_string()))? {
            if attempts >= MAX_ATTEMPTS {
                return Err(InviteError::Internal("Failed to generate unique invite code".to_string()));
            }
            code = Self::generate_unique_code();
            attempts += 1;
        }

        // Create invite entity
        let max_uses = request.max_uses.unwrap_or(0);
        let max_age = request.max_age.unwrap_or(86400); // Default: 24 hours
        let temporary = request.temporary.unwrap_or(false);

        let now = Utc::now();
        let expires_at = if max_age > 0 {
            Some(now + Duration::seconds(max_age as i64))
        } else {
            None
        };

        let invite = Invite {
            code,
            server_id: request.server_id,
            channel_id: request.channel_id,
            inviter_id: Some(inviter_id),
            max_uses,
            uses: 0,
            max_age,
            temporary,
            expires_at,
            created_at: now,
        };

        let created = self
            .invite_repo
            .create(&invite)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        Ok(InviteDto::from_invite(created))
    }

    async fn get_invite(&self, code: &str) -> Result<InviteDto, InviteError> {
        let invite = self
            .invite_repo
            .find_by_code(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?
            .ok_or(InviteError::NotFound)?;

        Ok(InviteDto::from_invite(invite))
    }

    async fn get_invite_preview(&self, code: &str) -> Result<InvitePreviewDto, InviteError> {
        // Get the invite
        let invite = self
            .invite_repo
            .find_by_code(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?
            .ok_or(InviteError::NotFound)?;

        let is_valid = invite.is_valid();

        // Get server info
        let guild = self
            .guild_service
            .get_guild(invite.server_id)
            .await
            .map_err(|_| InviteError::ServerNotFound)?;

        // For now, we don't have channel name lookup, so use a placeholder
        // A complete implementation would fetch channel details
        Ok(InvitePreviewDto {
            code: invite.code,
            server_id: invite.server_id.to_string(),
            server_name: guild.name,
            server_icon: guild.icon_url,
            channel_id: invite.channel_id.to_string(),
            channel_name: "general".to_string(), // Placeholder
            inviter_id: invite.inviter_id.map(|id| id.to_string()),
            inviter_name: None, // Would need user lookup
            member_count: guild.member_count,
            is_valid,
        })
    }

    async fn get_server_invites(&self, server_id: i64) -> Result<Vec<InviteDto>, InviteError> {
        let invites = self
            .invite_repo
            .find_by_server_id(server_id)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        Ok(invites.into_iter().map(InviteDto::from_invite).collect())
    }

    async fn use_invite(&self, code: &str, user_id: i64) -> Result<UseInviteResultDto, InviteError> {
        // Get and validate invite
        let invite = self
            .invite_repo
            .find_by_code(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?
            .ok_or(InviteError::NotFound)?;

        // Check if expired
        if invite.is_expired() {
            return Err(InviteError::Expired);
        }

        // Check if max uses reached
        if invite.is_maxed_out() {
            return Err(InviteError::MaxUsesReached);
        }

        // Check if already a member
        let is_member = self
            .member_repo
            .is_member(invite.server_id, user_id)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        if is_member {
            return Ok(UseInviteResultDto {
                server_id: invite.server_id.to_string(),
                already_member: true,
            });
        }

        // Increment invite uses
        self.invite_repo
            .increment_uses(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        // Join the guild
        self.guild_service
            .join_guild(invite.server_id, user_id)
            .await?;

        Ok(UseInviteResultDto {
            server_id: invite.server_id.to_string(),
            already_member: false,
        })
    }

    async fn delete_invite(&self, code: &str, actor_id: i64) -> Result<(), InviteError> {
        // Get invite
        let invite = self
            .invite_repo
            .find_by_code(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?
            .ok_or(InviteError::NotFound)?;

        // Check permission: must be inviter or have manage_invites permission
        let is_inviter = invite.inviter_id == Some(actor_id);
        let can_manage = self.can_manage_invites(invite.server_id, actor_id).await?;

        if !is_inviter && !can_manage {
            return Err(InviteError::Forbidden);
        }

        self.invite_repo
            .delete(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn validate_invite(&self, code: &str) -> Result<InviteValidationDto, InviteError> {
        let invite = self
            .invite_repo
            .find_by_code(code)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        match invite {
            Some(inv) => {
                let now = Utc::now();
                let is_expired = inv.is_expired();
                let is_maxed = inv.is_maxed_out();
                let is_valid = !is_expired && !is_maxed;

                let invalid_reason = if is_expired {
                    Some("Invite has expired".to_string())
                } else if is_maxed {
                    Some("Invite has reached maximum uses".to_string())
                } else {
                    None
                };

                let remaining_uses = inv.remaining_uses();

                let expires_in = inv.expires_at.map(|exp| {
                    let duration = exp - now;
                    duration.num_seconds().max(0)
                });

                Ok(InviteValidationDto {
                    code: inv.code,
                    is_valid,
                    invalid_reason,
                    remaining_uses,
                    expires_in,
                })
            }
            None => Ok(InviteValidationDto {
                code: code.to_string(),
                is_valid: false,
                invalid_reason: Some("Invite not found".to_string()),
                remaining_uses: None,
                expires_in: None,
            }),
        }
    }

    async fn get_user_invites(&self, user_id: i64) -> Result<Vec<InviteDto>, InviteError> {
        let invites = self
            .invite_repo
            .find_by_inviter_id(user_id)
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        Ok(invites.into_iter().map(InviteDto::from_invite).collect())
    }

    async fn cleanup_expired(&self) -> Result<u64, InviteError> {
        let deleted = self
            .invite_repo
            .delete_expired()
            .await
            .map_err(|e| InviteError::Internal(e.to_string()))?;

        Ok(deleted as u64)
    }
}

/// Concrete implementation using PostgreSQL repository.
pub type PgInviteService<G, M> = InviteServiceImpl<PgInviteRepository, G, M>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_invite_dto() {
        let dto = CreateInviteDto {
            server_id: 123,
            channel_id: 456,
            max_uses: Some(10),
            max_age: Some(3600),
            temporary: Some(false),
        };

        assert_eq!(dto.server_id, 123);
        assert_eq!(dto.channel_id, 456);
        assert_eq!(dto.max_uses, Some(10));
        assert_eq!(dto.max_age, Some(3600));
        assert_eq!(dto.temporary, Some(false));
    }

    #[test]
    fn test_invite_dto_from_invite() {
        let invite = Invite {
            code: "test1234".to_string(),
            server_id: 123,
            channel_id: 456,
            inviter_id: Some(789),
            max_uses: 10,
            uses: 5,
            max_age: 3600,
            temporary: false,
            expires_at: None,
            created_at: Utc::now(),
        };

        let dto = InviteDto::from_invite(invite);
        assert_eq!(dto.code, "test1234");
        assert_eq!(dto.server_id, "123");
        assert_eq!(dto.channel_id, "456");
        assert_eq!(dto.inviter_id, Some("789".to_string()));
        assert_eq!(dto.max_uses, 10);
        assert_eq!(dto.uses, 5);
        assert!(dto.is_valid);
    }

    #[test]
    fn test_invite_validation_dto() {
        let validation = InviteValidationDto {
            code: "test1234".to_string(),
            is_valid: true,
            invalid_reason: None,
            remaining_uses: Some(5),
            expires_in: Some(3600),
        };

        assert!(validation.is_valid);
        assert!(validation.invalid_reason.is_none());
        assert_eq!(validation.remaining_uses, Some(5));
    }
}
