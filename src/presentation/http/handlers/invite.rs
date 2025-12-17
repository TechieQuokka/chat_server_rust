//! Invite Handlers
//!
//! HTTP handlers for invite-related endpoints.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use crate::application::dto::request::CreateInviteRequest;
use crate::application::dto::response::{
    GuildResponse, InviteAcceptResponse, InviteChannelInfo, InviteGuildInfo, InvitePreviewResponse,
    InviteResponse, InviteUserInfo,
};
use crate::application::services::{
    CreateInviteDto, GuildService, GuildServiceImpl, InviteError, InviteService, InviteServiceImpl,
};
use crate::domain::{ChannelRepository, MemberRepository, ServerRepository};
use crate::infrastructure::repositories::{
    PgChannelRepository, PgInviteRepository, PgMemberRepository, PgRoleRepository,
    PgServerRepository, InviteRepository,
};
use crate::presentation::middleware::AuthUser;
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Helper to convert InviteError to AppError
fn map_invite_error(e: InviteError) -> AppError {
    match e {
        InviteError::NotFound => AppError::NotFound("Invite not found".into()),
        InviteError::Expired => AppError::BadRequest("Invite has expired".into()),
        InviteError::MaxUsesReached => AppError::BadRequest("Invite has reached maximum uses".into()),
        InviteError::InvalidCode => AppError::BadRequest("Invalid invite code".into()),
        InviteError::Forbidden => AppError::Forbidden("Permission denied".into()),
        InviteError::ServerNotFound => AppError::NotFound("Guild not found".into()),
        InviteError::ChannelNotFound => AppError::NotFound("Channel not found".into()),
        InviteError::AlreadyMember => AppError::Conflict("Already a member of this guild".into()),
        InviteError::Internal(msg) => AppError::Internal(msg),
    }
}

/// Create a new invite for a guild
///
/// POST /api/v1/guilds/:guild_id/invites
///
/// Creates an invite link for the specified guild. The invite can optionally
/// target a specific channel, have usage limits, and expire after a set time.
///
/// ## Request Body
/// - `channel_id` (optional): Channel ID for the invite. Defaults to first text channel.
/// - `max_uses` (optional): Maximum number of uses (0 = unlimited, max 100). Default: 0.
/// - `max_age` (optional): Duration in seconds before expiration (0 = never, max 604800). Default: 86400 (24h).
/// - `temporary` (optional): Whether members are kicked when they go offline. Default: false.
///
/// ## Permissions Required
/// - Must be a member of the guild (CREATE_INSTANT_INVITE permission by default)
pub async fn create_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(guild_id): Path<String>,
    Json(body): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Parse optional channel_id
    let channel_id = body
        .channel_id
        .as_ref()
        .map(|id| {
            id.parse::<i64>()
                .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))
        })
        .transpose()?;

    // Create service dependencies
    let invite_repo = Arc::new(PgInviteRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service: Arc<
        GuildServiceImpl<PgServerRepository, PgChannelRepository, PgMemberRepository, PgRoleRepository>,
    > = Arc::new(GuildServiceImpl::new(
        server_repo.clone(),
        channel_repo.clone(),
        member_repo.clone(),
        role_repo,
        state.snowflake.clone(),
    ));

    let invite_service = InviteServiceImpl::new(invite_repo, guild_service, member_repo);

    // Get first channel if not specified
    let final_channel_id = match channel_id {
        Some(id) => id,
        None => {
            // Get channels for the guild and use the first text channel
            let channels = channel_repo
                .find_by_server_id(guild_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            channels
                .into_iter()
                .find(|c| c.channel_type.as_str() == "text")
                .map(|c| c.id)
                .ok_or_else(|| AppError::NotFound("No text channel found in guild".into()))?
        }
    };

    let request = CreateInviteDto {
        server_id: guild_id,
        channel_id: final_channel_id,
        max_uses: Some(body.max_uses),
        max_age: Some(body.max_age),
        temporary: Some(body.temporary),
    };

    let invite_dto = invite_service
        .create_invite(request, auth.user_id)
        .await
        .map_err(map_invite_error)?;

    // Get additional info for response
    let server = server_repo
        .find_by_id(guild_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Guild not found".into()))?;

    let channel = channel_repo
        .find_by_id(final_channel_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Channel not found".into()))?;

    let response = InviteResponse {
        code: invite_dto.code,
        guild: InviteGuildInfo {
            id: server.id.to_string(),
            name: server.name,
            icon_url: server.icon_url,
        },
        channel: InviteChannelInfo {
            id: channel.id.to_string(),
            name: channel.name,
            channel_type: channel.channel_type.as_str().to_string(),
        },
        inviter: Some(InviteUserInfo {
            id: auth.user_id.to_string(),
            username: "".to_string(), // Would need user lookup for username
            avatar_url: None,
        }),
        max_uses: invite_dto.max_uses,
        uses: invite_dto.uses,
        max_age: invite_dto.max_age,
        temporary: invite_dto.temporary,
        expires_at: invite_dto.expires_at,
        created_at: invite_dto.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get invite details by code (public preview)
///
/// GET /api/v1/invites/:code
///
/// Returns public information about an invite that can be shown before
/// a user decides to join. Does not require authentication.
///
/// ## Path Parameters
/// - `code`: The invite code (e.g., "aBcD1234")
pub async fn get_invite(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<InvitePreviewResponse>, AppError> {
    let invite_repo = Arc::new(PgInviteRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service: Arc<
        GuildServiceImpl<PgServerRepository, PgChannelRepository, PgMemberRepository, PgRoleRepository>,
    > = Arc::new(GuildServiceImpl::new(
        server_repo.clone(),
        channel_repo.clone(),
        member_repo.clone(),
        role_repo,
        state.snowflake.clone(),
    ));

    let invite_service = InviteServiceImpl::new(invite_repo, guild_service, member_repo);

    let preview = invite_service
        .get_invite_preview(&code)
        .await
        .map_err(map_invite_error)?;

    let response = InvitePreviewResponse {
        code: preview.code,
        guild: InviteGuildInfo {
            id: preview.server_id.clone(),
            name: preview.server_name,
            icon_url: preview.server_icon,
        },
        channel: InviteChannelInfo {
            id: preview.channel_id,
            name: preview.channel_name,
            channel_type: "text".to_string(), // Default for now
        },
        inviter: preview.inviter_id.map(|id| InviteUserInfo {
            id,
            username: preview.inviter_name.unwrap_or_default(),
            avatar_url: None,
        }),
        approximate_member_count: preview.member_count,
        expires_at: None, // Preview doesn't include expiry info currently
    };

    Ok(Json(response))
}

/// Use/accept an invite (join the server)
///
/// POST /api/v1/invites/:code
///
/// Uses the invite to join the associated server. The invite's usage count
/// is incremented and the user becomes a member of the guild.
///
/// ## Path Parameters
/// - `code`: The invite code (e.g., "aBcD1234")
///
/// ## Errors
/// - 404: Invite not found
/// - 400: Invite expired or max uses reached
/// - 409: Already a member of this guild
pub async fn accept_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(code): Path<String>,
) -> Result<Json<InviteAcceptResponse>, AppError> {
    let invite_repo = Arc::new(PgInviteRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service: Arc<
        GuildServiceImpl<PgServerRepository, PgChannelRepository, PgMemberRepository, PgRoleRepository>,
    > = Arc::new(GuildServiceImpl::new(
        server_repo.clone(),
        channel_repo.clone(),
        member_repo.clone(),
        role_repo,
        state.snowflake.clone(),
    ));

    let invite_service = InviteServiceImpl::new(invite_repo, guild_service.clone(), member_repo.clone());

    let result = invite_service
        .use_invite(&code, auth.user_id)
        .await
        .map_err(map_invite_error)?;

    // If already a member, return the guild info anyway
    let guild_id: i64 = result.server_id.parse().map_err(|_| AppError::Internal("Invalid server ID".into()))?;

    let guild_dto = guild_service
        .get_guild(guild_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let response = InviteAcceptResponse {
        guild: GuildResponse {
            id: guild_dto.id,
            name: guild_dto.name,
            owner_id: guild_dto.owner_id,
            icon_url: guild_dto.icon_url,
            description: guild_dto.description,
            member_count: guild_dto.member_count,
            created_at: guild_dto.created_at,
        },
    };

    Ok(Json(response))
}

/// Delete an invite
///
/// DELETE /api/v1/invites/:code
///
/// Deletes an invite by its code. Requires MANAGE_GUILD permission or
/// being the original invite creator.
///
/// ## Path Parameters
/// - `code`: The invite code (e.g., "aBcD1234")
///
/// ## Permissions Required
/// - MANAGE_GUILD permission, OR
/// - Be the user who created the invite
pub async fn delete_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(code): Path<String>,
) -> Result<StatusCode, AppError> {
    let invite_repo = Arc::new(PgInviteRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service: Arc<
        GuildServiceImpl<PgServerRepository, PgChannelRepository, PgMemberRepository, PgRoleRepository>,
    > = Arc::new(GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo.clone(),
        role_repo,
        state.snowflake.clone(),
    ));

    let invite_service = InviteServiceImpl::new(invite_repo, guild_service, member_repo);

    invite_service
        .delete_invite(&code, auth.user_id)
        .await
        .map_err(map_invite_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List all invites for a guild
///
/// GET /api/v1/guilds/:guild_id/invites
///
/// Returns all invites for the specified guild. Requires MANAGE_GUILD permission.
///
/// ## Path Parameters
/// - `guild_id`: The guild ID
///
/// ## Permissions Required
/// - MANAGE_GUILD permission
pub async fn list_guild_invites(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<InviteResponse>>, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let invite_repo = Arc::new(PgInviteRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    // Verify user has permission (must be a member for now)
    let is_member = member_repo
        .is_member(guild_id, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !is_member {
        return Err(AppError::Forbidden("Permission denied".into()));
    }

    // Get guild info
    let server = server_repo
        .find_by_id(guild_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Guild not found".into()))?;

    // Get all invites
    let invites = invite_repo
        .find_by_server_id(guild_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut responses = Vec::with_capacity(invites.len());

    for invite in invites {
        // Get channel info
        let channel = channel_repo
            .find_by_id(invite.channel_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        responses.push(InviteResponse {
            code: invite.code,
            guild: InviteGuildInfo {
                id: server.id.to_string(),
                name: server.name.clone(),
                icon_url: server.icon_url.clone(),
            },
            channel: InviteChannelInfo {
                id: channel.as_ref().map(|c| c.id.to_string()).unwrap_or_default(),
                name: channel.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
                channel_type: channel
                    .as_ref()
                    .map(|c| c.channel_type.as_str().to_string())
                    .unwrap_or_else(|| "text".to_string()),
            },
            inviter: invite.inviter_id.map(|id| InviteUserInfo {
                id: id.to_string(),
                username: String::new(), // Would need user lookup
                avatar_url: None,
            }),
            max_uses: invite.max_uses,
            uses: invite.uses,
            max_age: invite.max_age,
            temporary: invite.temporary,
            expires_at: invite.expires_at.map(|e| e.to_rfc3339()),
            created_at: invite.created_at.to_rfc3339(),
        });
    }

    Ok(Json(responses))
}
