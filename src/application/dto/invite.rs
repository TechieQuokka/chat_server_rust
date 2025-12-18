//! Invite DTOs
//!
//! Data Transfer Objects for invite-related API operations.

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::application::services::{InviteDto, InvitePreviewDto, InviteValidationDto, UseInviteResultDto};

/// Request to create a new invite.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateInviteRequest {
    /// Channel ID the invite leads to.
    pub channel_id: String,

    /// Maximum number of uses (0 = unlimited).
    /// Common values: 1, 5, 10, 25, 50, 100
    #[validate(range(min = 0, max = 1000, message = "max_uses must be between 0 and 1000"))]
    pub max_uses: Option<i32>,

    /// Seconds until expiration (0 = never expires).
    /// Common values: 1800 (30 min), 3600 (1 hour), 21600 (6 hours),
    /// 43200 (12 hours), 86400 (24 hours), 604800 (7 days), 0 (never)
    #[validate(range(min = 0, max = 604_800, message = "max_age must be between 0 and 604800 (7 days)"))]
    pub max_age: Option<i32>,

    /// Whether members gain temporary membership.
    /// Temporary members are kicked when they go offline.
    pub temporary: Option<bool>,
}

/// Response for a created or retrieved invite.
#[derive(Debug, Serialize)]
pub struct InviteResponse {
    /// Unique invite code.
    pub code: String,
    /// Server ID this invite is for.
    pub guild_id: String,
    /// Channel ID the invite leads to.
    pub channel_id: String,
    /// User ID who created the invite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inviter_id: Option<String>,
    /// Maximum uses (0 = unlimited).
    pub max_uses: i32,
    /// Current use count.
    pub uses: i32,
    /// Max age in seconds (0 = never).
    pub max_age: i32,
    /// Whether membership is temporary.
    pub temporary: bool,
    /// Expiration timestamp (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Creation timestamp (ISO 8601).
    pub created_at: String,
}

impl From<InviteDto> for InviteResponse {
    fn from(dto: InviteDto) -> Self {
        Self {
            code: dto.code,
            guild_id: dto.server_id,
            channel_id: dto.channel_id,
            inviter_id: dto.inviter_id,
            max_uses: dto.max_uses,
            uses: dto.uses,
            max_age: dto.max_age,
            temporary: dto.temporary,
            expires_at: dto.expires_at,
            created_at: dto.created_at,
        }
    }
}

/// Response for invite preview (before joining).
#[derive(Debug, Serialize)]
pub struct InvitePreviewResponse {
    /// Invite code.
    pub code: String,
    /// Server information.
    pub guild: InviteGuildPreview,
    /// Channel information.
    pub channel: InviteChannelPreview,
    /// Inviter information (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inviter: Option<InviteInviterPreview>,
    /// Approximate member count.
    pub approximate_member_count: i64,
}

/// Guild preview in invite response.
#[derive(Debug, Serialize)]
pub struct InviteGuildPreview {
    /// Server ID.
    pub id: String,
    /// Server name.
    pub name: String,
    /// Server icon URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Channel preview in invite response.
#[derive(Debug, Serialize)]
pub struct InviteChannelPreview {
    /// Channel ID.
    pub id: String,
    /// Channel name.
    pub name: String,
}

/// Inviter preview in invite response.
#[derive(Debug, Serialize)]
pub struct InviteInviterPreview {
    /// User ID.
    pub id: String,
    /// Username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

impl From<InvitePreviewDto> for InvitePreviewResponse {
    fn from(dto: InvitePreviewDto) -> Self {
        Self {
            code: dto.code,
            guild: InviteGuildPreview {
                id: dto.server_id,
                name: dto.server_name,
                icon: dto.server_icon,
            },
            channel: InviteChannelPreview {
                id: dto.channel_id,
                name: dto.channel_name,
            },
            inviter: dto.inviter_id.map(|id| InviteInviterPreview {
                id,
                username: dto.inviter_name,
            }),
            approximate_member_count: dto.member_count,
        }
    }
}

/// Response for using an invite.
#[derive(Debug, Serialize)]
pub struct UseInviteResponse {
    /// Server ID joined.
    pub guild_id: String,
    /// Whether user was already a member.
    pub new_member: bool,
}

impl From<UseInviteResultDto> for UseInviteResponse {
    fn from(dto: UseInviteResultDto) -> Self {
        Self {
            guild_id: dto.server_id,
            new_member: !dto.already_member,
        }
    }
}

/// Response for invite validation.
#[derive(Debug, Serialize)]
pub struct InviteValidationResponse {
    /// Invite code.
    pub code: String,
    /// Whether invite is valid.
    pub valid: bool,
    /// Reason if invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Remaining uses (null if unlimited).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_uses: Option<i32>,
    /// Seconds until expiration (null if never).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
}

impl From<InviteValidationDto> for InviteValidationResponse {
    fn from(dto: InviteValidationDto) -> Self {
        Self {
            code: dto.code,
            valid: dto.is_valid,
            reason: dto.invalid_reason,
            remaining_uses: dto.remaining_uses,
            expires_in_seconds: dto.expires_in,
        }
    }
}

/// List of invites response.
#[derive(Debug, Serialize)]
pub struct InviteListResponse {
    /// List of invites.
    pub invites: Vec<InviteResponse>,
}

impl From<Vec<InviteDto>> for InviteListResponse {
    fn from(dtos: Vec<InviteDto>) -> Self {
        Self {
            invites: dtos.into_iter().map(InviteResponse::from).collect(),
        }
    }
}
