//! Channel Handlers

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use crate::application::dto::request::{CreateChannelRequest, UpdateChannelRequest};
use crate::application::dto::response::ChannelResponse;
use crate::application::services::{
    ChannelError, ChannelService, ChannelServiceImpl, CreateChannelDto, UpdateChannelDto,
};
use crate::infrastructure::repositories::{
    PgChannelRepository, PgMemberRepository, PgServerRepository,
};
use crate::presentation::middleware::AuthUser;
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Create a new channel
pub async fn create_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(guild_id): Path<String>,
    Json(body): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), AppError> {
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let channel_service = ChannelServiceImpl::new(
        channel_repo,
        server_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let request = CreateChannelDto {
        name: body.name,
        channel_type: body.channel_type,
        topic: body.topic,
        parent_id: body.parent_id.and_then(|s| s.parse().ok()),
        position: body.position,
        nsfw: body.nsfw,
    };

    let channel = channel_service
        .create_channel(guild_id, auth.user_id, request)
        .await
        .map_err(|e| match e {
            ChannelError::GuildNotFound => AppError::NotFound("Guild not found".into()),
            ChannelError::Forbidden => AppError::Forbidden("Permission denied".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(ChannelResponse::from(channel))))
}

/// Get channel by ID
pub async fn get_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<ChannelResponse>, AppError> {
    let channel_id: i64 = channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let channel_service = ChannelServiceImpl::new(
        channel_repo,
        server_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let channel = channel_service
        .get_channel(channel_id)
        .await
        .map_err(|e| match e {
            ChannelError::NotFound => AppError::NotFound("Channel not found".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(ChannelResponse::from(channel)))
}

/// Update channel
pub async fn update_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(channel_id): Path<String>,
    Json(body): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, AppError> {
    let channel_id: i64 = channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let channel_service = ChannelServiceImpl::new(
        channel_repo,
        server_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let update = UpdateChannelDto {
        name: body.name,
        topic: body.topic,
        position: body.position,
        parent_id: body.parent_id.map(|opt| opt.and_then(|s| s.parse().ok())),
        nsfw: body.nsfw,
        rate_limit_per_user: body.rate_limit_per_user,
    };

    let channel = channel_service
        .update_channel(channel_id, auth.user_id, update)
        .await
        .map_err(|e| match e {
            ChannelError::NotFound => AppError::NotFound("Channel not found".into()),
            ChannelError::Forbidden => AppError::Forbidden("Permission denied".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(ChannelResponse::from(channel)))
}

/// Delete channel
pub async fn delete_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(channel_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let channel_id: i64 = channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let channel_service = ChannelServiceImpl::new(
        channel_repo,
        server_repo,
        member_repo,
        state.snowflake.clone(),
    );

    channel_service
        .delete_channel(channel_id, auth.user_id)
        .await
        .map_err(|e| match e {
            ChannelError::NotFound => AppError::NotFound("Channel not found".into()),
            ChannelError::Forbidden => AppError::Forbidden("Permission denied".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(StatusCode::NO_CONTENT)
}
