//! Message Handlers

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::application::dto::request::SendMessageRequest;
use crate::application::dto::response::MessageResponse;
use crate::application::services::{
    CreateMessageDto, MessageError, MessageQueryDto, MessageService, MessageServiceImpl,
};
use crate::infrastructure::repositories::{
    PgChannelRepository, PgMemberRepository, PgMessageRepository,
};
use crate::presentation::middleware::AuthUser;
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Message query parameters
#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub before: Option<String>,
    pub after: Option<String>,
    pub around: Option<String>,
    pub limit: Option<i32>,
}

/// Get messages from channel
pub async fn get_messages(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, AppError> {
    let channel_id: i64 = channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    let message_repo = Arc::new(PgMessageRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let message_service = MessageServiceImpl::new(
        message_repo,
        channel_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let query_dto = MessageQueryDto {
        before: query.before.and_then(|s| s.parse().ok()),
        after: query.after.and_then(|s| s.parse().ok()),
        around: query.around.and_then(|s| s.parse().ok()),
        limit: query.limit,
    };

    let messages = message_service
        .get_messages(channel_id, query_dto)
        .await
        .map_err(|e| match e {
            MessageError::ChannelNotFound => AppError::NotFound("Channel not found".into()),
            e => AppError::Internal(e.to_string()),
        })?;

    let responses: Vec<MessageResponse> = messages.into_iter().map(MessageResponse::from).collect();

    Ok(Json(responses))
}

/// Send message to channel
pub async fn send_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(channel_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), AppError> {
    let channel_id: i64 = channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let message_repo = Arc::new(PgMessageRepository::new(state.db.clone()));
    let channel_repo = Arc::new(PgChannelRepository::new(state.db.clone()));
    let member_repo = Arc::new(PgMemberRepository::new(state.db.clone()));

    let message_service = MessageServiceImpl::new(
        message_repo,
        channel_repo,
        member_repo,
        state.snowflake.clone(),
    );

    let request = CreateMessageDto {
        content: body.content,
        reply_to: body.reply_to.and_then(|s| s.parse().ok()),
    };

    let message = message_service
        .send_message(channel_id, auth.user_id, request)
        .await
        .map_err(|e| match e {
            MessageError::ChannelNotFound => AppError::NotFound("Channel not found".into()),
            MessageError::Forbidden => AppError::Forbidden("Permission denied".into()),
            MessageError::ContentTooLong => {
                AppError::BadRequest("Message content too long (max 2000 characters)".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(MessageResponse::from(message))))
}
