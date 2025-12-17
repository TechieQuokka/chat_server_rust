//! Guild Handlers

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use crate::application::dto::request::{CreateGuildRequest, MembersQueryParams, UpdateGuildRequest};
use crate::application::dto::response::{ChannelResponse, GuildResponse, MemberResponse};
use crate::application::services::{
    ChannelService, ChannelServiceImpl, CreateGuildDto, GuildError, GuildService,
    GuildServiceImpl, UpdateGuildDto,
};
use crate::infrastructure::repositories::{
    PgChannelRepository, PgMemberRepository, PgRoleRepository, PgServerRepository,
};
use crate::presentation::middleware::AuthUser;
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Create a new guild
pub async fn create_guild(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateGuildRequest>,
) -> Result<(StatusCode, Json<GuildResponse>), AppError> {
    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service = GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo,
        role_repo,
        state.snowflake.clone(),
    );

    let request = CreateGuildDto {
        name: body.name,
        icon_url: body.icon_url,
        description: body.description,
    };

    let guild = guild_service
        .create_guild(auth.user_id, request)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(GuildResponse::from(guild))))
}

/// Get guild by ID
pub async fn get_guild(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<GuildResponse>, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service = GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo,
        role_repo,
        state.snowflake.clone(),
    );

    let guild = guild_service
        .get_guild(guild_id)
        .await
        .map_err(|e| match e {
            GuildError::NotFound => AppError::NotFound("Guild not found".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(GuildResponse::from(guild)))
}

/// Update guild
pub async fn update_guild(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(guild_id): Path<String>,
    Json(body): Json<UpdateGuildRequest>,
) -> Result<Json<GuildResponse>, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service = GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo,
        role_repo,
        state.snowflake.clone(),
    );

    let update = UpdateGuildDto {
        name: body.name,
        icon_url: body.icon_url,
        description: body.description,
    };

    let guild = guild_service
        .update_guild(guild_id, auth.user_id, update)
        .await
        .map_err(|e| match e {
            GuildError::NotFound => AppError::NotFound("Guild not found".into()),
            GuildError::Forbidden => AppError::Forbidden("Permission denied".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(GuildResponse::from(guild)))
}

/// Delete guild
pub async fn delete_guild(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(guild_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service = GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo,
        role_repo,
        state.snowflake.clone(),
    );

    guild_service
        .delete_guild(guild_id, auth.user_id)
        .await
        .map_err(|e| match e {
            GuildError::NotFound => AppError::NotFound("Guild not found".into()),
            GuildError::Forbidden => AppError::Forbidden("Permission denied".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get guild channels
pub async fn get_guild_channels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<ChannelResponse>>, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let channel_service = ChannelServiceImpl::new(
        channel_repo,
        server_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let channels = channel_service
        .get_guild_channels(guild_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let responses: Vec<ChannelResponse> = channels.into_iter().map(ChannelResponse::from).collect();

    Ok(Json(responses))
}

/// Get guild members
pub async fn get_guild_members(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<MembersQueryParams>,
) -> Result<Json<Vec<MemberResponse>>, AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let after = params.after.and_then(|s| s.parse::<i64>().ok());
    let limit = params.limit.unwrap_or(100).min(1000);

    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));
    let role_repo = Arc::new(PgRoleRepository::new(state.db.clone()));

    let guild_service = GuildServiceImpl::new(
        server_repo,
        channel_repo,
        member_repo,
        role_repo,
        state.snowflake.clone(),
    );

    let members = guild_service
        .get_members(guild_id, after, limit)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let responses: Vec<MemberResponse> = members.into_iter().map(MemberResponse::from).collect();

    Ok(Json(responses))
}
