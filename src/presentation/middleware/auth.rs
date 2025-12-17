//! Authentication Middleware
//!
//! JWT validation middleware for protected routes.

use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;
use crate::startup::AppState;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at time (Unix timestamp)
    pub iat: i64,
}

/// Authenticated user extension
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
}

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".into()))?;

    // Check for Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".into()))?;

    // Decode and validate JWT
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.settings.jwt.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::Unauthorized("Token expired".into())
        }
        _ => AppError::Unauthorized("Invalid token".into()),
    })?;

    // Parse user ID from claims
    let user_id: i64 = token_data
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token claims".into()))?;

    // Insert authenticated user into request extensions
    request.extensions_mut().insert(AuthUser { user_id });

    // Continue to the next handler
    Ok(next.run(request).await)
}

/// Optional authentication middleware (doesn't fail if no token)
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token
    if let Some(auth_header) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(token_data) = decode::<Claims>(
                token,
                &DecodingKey::from_secret(state.settings.jwt.secret.as_bytes()),
                &Validation::default(),
            ) {
                if let Ok(user_id) = token_data.claims.sub.parse::<i64>() {
                    request.extensions_mut().insert(AuthUser { user_id });
                }
            }
        }
    }

    next.run(request).await
}
