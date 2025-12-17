//! Request DTOs
//!
//! Data structures for API request bodies.

use serde::Deserialize;
use validator::Validate;

use crate::shared::validation::validate_password_strength;

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

    #[validate(custom(function = "validate_password_strength"))]
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

/// Create invite request
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateInviteRequest {
    /// Channel ID to create the invite for (optional, defaults to first text channel)
    pub channel_id: Option<String>,

    /// Maximum number of uses (0 = unlimited, max 100)
    #[validate(range(min = 0, max = 100, message = "max_uses must be between 0 and 100"))]
    #[serde(default)]
    pub max_uses: i32,

    /// Duration in seconds before expiration (0 = never, max 604800 = 7 days)
    #[validate(range(min = 0, max = 604800, message = "max_age must be between 0 and 604800 seconds"))]
    #[serde(default)]
    pub max_age: i32,

    /// Whether this invite grants temporary membership
    #[serde(default)]
    pub temporary: bool,
}

// =============================================================================
// Role DTOs
// =============================================================================

/// Create role request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    /// Permission bitfield (as string for JavaScript BigInt compatibility)
    pub permissions: Option<String>,

    /// Role color as RGB integer
    pub color: Option<i32>,

    /// Whether to display role members separately in the member list
    pub hoist: Option<bool>,

    /// Whether this role can be mentioned by everyone
    pub mentionable: Option<bool>,
}

/// Update role request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,

    /// Permission bitfield (as string for JavaScript BigInt compatibility)
    pub permissions: Option<String>,

    /// Role color as RGB integer (use null to remove color)
    pub color: Option<Option<i32>>,

    /// Whether to display role members separately
    pub hoist: Option<bool>,

    /// Whether this role can be mentioned
    pub mentionable: Option<bool>,
}

/// Role position for reordering
#[derive(Debug, Deserialize)]
pub struct RolePositionRequest {
    /// Role ID
    pub id: String,

    /// New position
    pub position: i32,
}

/// Reorder roles request
#[derive(Debug, Deserialize)]
pub struct ReorderRolesRequest {
    /// List of role positions
    pub positions: Vec<RolePositionRequest>,
}

/// Assign role to member request
#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    /// User ID to assign the role to
    pub user_id: String,
}
