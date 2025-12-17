//! User Handlers

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use validator::Validate;

use crate::application::dto::request::UpdateUserRequest;
use crate::application::dto::response::UserResponse;
use crate::application::services::{ServerPreviewDto, UpdateProfileDto, UserService, UserServiceImpl};
use crate::infrastructure::repositories::{PgServerRepository, PgUserRepository};
use crate::presentation::middleware::AuthUser;
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Get current authenticated user
pub async fn get_current_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let user_service = UserServiceImpl::new(user_repo, server_repo);

    let user = user_service
        .get_user(auth.user_id)
        .await
        .map_err(|e| match e {
            crate::application::services::UserError::NotFound => {
                AppError::NotFound("User not found".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(UserResponse::from_dto(user, true)))
}

/// Update current user profile
pub async fn update_current_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let user_service = UserServiceImpl::new(user_repo, server_repo);

    let update = UpdateProfileDto {
        username: body.username,
        display_name: body.display_name,
        avatar_url: body.avatar_url,
        bio: body.bio,
    };

    let user = user_service
        .update_profile(auth.user_id, update)
        .await
        .map_err(|e| match e {
            crate::application::services::UserError::NotFound => {
                AppError::NotFound("User not found".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(UserResponse::from_dto(user, true)))
}

/// Server preview for user's guild list
#[derive(Debug, serde::Serialize)]
pub struct ServerPreviewResponse {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner: bool,
}

impl From<ServerPreviewDto> for ServerPreviewResponse {
    fn from(dto: ServerPreviewDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            icon_url: dto.icon_url,
            owner: dto.owner,
        }
    }
}

/// Get user's guilds
pub async fn get_user_guilds(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<ServerPreviewResponse>>, AppError> {
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let user_service = UserServiceImpl::new(user_repo, server_repo);

    let guilds = user_service
        .get_user_servers(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let responses: Vec<ServerPreviewResponse> = guilds.into_iter().map(ServerPreviewResponse::from).collect();

    Ok(Json(responses))
}

/// Get user by ID
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, AppError> {
    let user_id: i64 = user_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let server_repo = Arc::new(PgServerRepository::new(state.db.clone()));
    let user_service = UserServiceImpl::new(user_repo, server_repo);

    let user = user_service
        .get_user(user_id)
        .await
        .map_err(|e| match e {
            crate::application::services::UserError::NotFound => {
                AppError::NotFound("User not found".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    // Don't include email for other users
    Ok(Json(UserResponse::from_dto(user, false)))
}
