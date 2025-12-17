//! Request DTOs
//!
//! Data structures for API request bodies.

use serde::Deserialize;
use validator::Validate;

/// Login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

/// Registration request
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 2, max = 32, message = "Username must be 2-32 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Update user request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 2, max = 32, message = "Username must be 2-32 characters"))]
    pub username: Option<String>,

    #[validate(length(max = 32, message = "Display name must be at most 32 characters"))]
    pub display_name: Option<String>,

    pub avatar_url: Option<String>,

    #[validate(length(max = 190, message = "Bio must be at most 190 characters"))]
    pub bio: Option<String>,
}

/// Create guild request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateGuildRequest {
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: String,

    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// Update guild request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateGuildRequest {
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: Option<String>,

    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// Create channel request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateChannelRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    #[serde(rename = "type")]
    pub channel_type: Option<String>,

    pub topic: Option<String>,
    pub parent_id: Option<String>,
    pub position: Option<i32>,
    pub nsfw: Option<bool>,
}

/// Update channel request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateChannelRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,

    pub topic: Option<String>,
    pub position: Option<i32>,
    pub parent_id: Option<Option<String>>,
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<i32>,
}

/// Send message request
#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    #[validate(length(min = 1, max = 2000, message = "Content must be 1-2000 characters"))]
    pub content: String,

    pub reply_to: Option<String>,

    #[serde(default)]
    pub attachments: Vec<String>,
}

/// Message query parameters
#[derive(Debug, Deserialize)]
pub struct MessageQueryParams {
    pub before: Option<String>,
    pub after: Option<String>,
    pub around: Option<String>,
    pub limit: Option<i32>,
}

/// Guild members query parameters
#[derive(Debug, Deserialize)]
pub struct MembersQueryParams {
    pub after: Option<String>,
    pub limit: Option<i32>,
}
