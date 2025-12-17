//! Authentication Handlers

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use validator::Validate;

use crate::application::dto::request::{LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::application::dto::response::{RegisterResponse, TokenResponse, UserResponse};
use crate::application::services::{AuthService, AuthServiceImpl};
use crate::config::JwtSettings;
use crate::infrastructure::repositories::{PgSessionRepository, PgUserRepository};
use crate::shared::error::AppError;
use crate::startup::AppState;

/// Register a new user
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create service
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let session_repo = Arc::new(PgSessionRepository::new(state.db.clone()));
    let jwt_settings = JwtSettings {
        secret: state.settings.jwt.secret.clone(),
        access_token_expiry_minutes: state.settings.jwt.access_token_expiry_minutes,
        refresh_token_expiry_days: state.settings.jwt.refresh_token_expiry_days,
    };
    let auth_service = AuthServiceImpl::new(
        user_repo,
        session_repo,
        state.snowflake.clone(),
        jwt_settings,
    );

    // Register user
    let (user, tokens) = auth_service
        .register(&body.username, &body.email, &body.password)
        .await
        .map_err(|e| match e {
            crate::application::services::AuthError::EmailExists => {
                AppError::Conflict("Email already exists".into())
            }
            crate::application::services::AuthError::UsernameExists => {
                AppError::Conflict("Username already exists".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    let response = RegisterResponse {
        user: UserResponse::from_user(user, true),
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        token_type: tokens.token_type,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Login with credentials
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // Validate request
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create service
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let session_repo = Arc::new(PgSessionRepository::new(state.db.clone()));
    let jwt_settings = JwtSettings {
        secret: state.settings.jwt.secret.clone(),
        access_token_expiry_minutes: state.settings.jwt.access_token_expiry_minutes,
        refresh_token_expiry_days: state.settings.jwt.refresh_token_expiry_days,
    };
    let auth_service = AuthServiceImpl::new(
        user_repo,
        session_repo,
        state.snowflake.clone(),
        jwt_settings,
    );

    // Authenticate
    let tokens = auth_service
        .authenticate(&body.email, &body.password)
        .await
        .map_err(|e| match e {
            crate::application::services::AuthError::InvalidCredentials => {
                AppError::Unauthorized("Invalid email or password".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(TokenResponse::from(tokens)))
}

/// Refresh access token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // Create service
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let session_repo = Arc::new(PgSessionRepository::new(state.db.clone()));
    let jwt_settings = JwtSettings {
        secret: state.settings.jwt.secret.clone(),
        access_token_expiry_minutes: state.settings.jwt.access_token_expiry_minutes,
        refresh_token_expiry_days: state.settings.jwt.refresh_token_expiry_days,
    };
    let auth_service = AuthServiceImpl::new(
        user_repo,
        session_repo,
        state.snowflake.clone(),
        jwt_settings,
    );

    // Refresh token
    let tokens = auth_service
        .refresh_token(&body.refresh_token)
        .await
        .map_err(|e| match e {
            crate::application::services::AuthError::SessionNotFound => {
                AppError::Unauthorized("Invalid or expired refresh token".into())
            }
            crate::application::services::AuthError::TokenExpired => {
                AppError::Unauthorized("Refresh token expired".into())
            }
            e => AppError::Internal(e.to_string()),
        })?;

    Ok(Json(TokenResponse::from(tokens)))
}

/// Logout (revoke refresh token)
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenRequest>,
) -> Result<StatusCode, AppError> {
    // Create service
    let user_repo = Arc::new(PgUserRepository::new(state.db.clone()));
    let session_repo = Arc::new(PgSessionRepository::new(state.db.clone()));
    let jwt_settings = JwtSettings {
        secret: state.settings.jwt.secret.clone(),
        access_token_expiry_minutes: state.settings.jwt.access_token_expiry_minutes,
        refresh_token_expiry_days: state.settings.jwt.refresh_token_expiry_days,
    };
    let auth_service = AuthServiceImpl::new(
        user_repo,
        session_repo,
        state.snowflake.clone(),
        jwt_settings,
    );

    // Revoke token (ignore errors for logout)
    let _ = auth_service.revoke_token(&body.refresh_token).await;

    Ok(StatusCode::NO_CONTENT)
}
