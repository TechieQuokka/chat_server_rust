//! Response DTOs
//!
//! Data structures for API response bodies.

use serde::Serialize;

use crate::application::services::{AuthTokens, UserDto, GuildDto, ChannelDto, MessageDto, MemberDto};
use crate::domain::User;

/// Authentication tokens response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

impl From<AuthTokens> for TokenResponse {
    fn from(tokens: AuthTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            token_type: tokens.token_type,
        }
    }
}

/// Registration response (includes user and tokens)
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// User response
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub bio: Option<String>,
    pub created_at: String,
}

impl UserResponse {
    pub fn from_user(user: User, include_email: bool) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            email: if include_email { Some(user.email) } else { None },
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            status: user.status.as_str().to_string(),
            bio: user.bio,
            created_at: user.created_at.to_rfc3339(),
        }
    }

    pub fn from_dto(dto: UserDto, include_email: bool) -> Self {
        Self {
            id: dto.id,
            username: dto.username,
            email: if include_email { Some(dto.email) } else { None },
            display_name: dto.display_name,
            avatar_url: dto.avatar_url,
            status: dto.status,
            bio: dto.bio,
            created_at: dto.created_at,
        }
    }
}

/// Guild response
#[derive(Debug, Serialize)]
pub struct GuildResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
    pub member_count: i64,
    pub created_at: String,
}

impl From<GuildDto> for GuildResponse {
    fn from(dto: GuildDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            owner_id: dto.owner_id,
            icon_url: dto.icon_url,
            description: dto.description,
            member_count: dto.member_count,
            created_at: dto.created_at,
        }
    }
}

/// Channel response
#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub guild_id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub topic: Option<String>,
    pub position: i32,
    pub parent_id: Option<String>,
    pub nsfw: bool,
    pub rate_limit_per_user: i32,
    pub created_at: String,
}

impl From<ChannelDto> for ChannelResponse {
    fn from(dto: ChannelDto) -> Self {
        Self {
            id: dto.id,
            guild_id: dto.guild_id,
            name: dto.name,
            channel_type: dto.channel_type,
            topic: dto.topic,
            position: dto.position,
            parent_id: dto.parent_id,
            nsfw: dto.nsfw,
            rate_limit_per_user: dto.rate_limit_per_user,
            created_at: dto.created_at,
        }
    }
}

/// Message response
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub reply_to_id: Option<String>,
    pub pinned: bool,
    pub edited_at: Option<String>,
    pub created_at: String,
}

impl From<MessageDto> for MessageResponse {
    fn from(dto: MessageDto) -> Self {
        Self {
            id: dto.id,
            channel_id: dto.channel_id,
            author_id: dto.author_id,
            content: dto.content,
            message_type: dto.message_type,
            reply_to_id: dto.reply_to_id,
            pinned: dto.pinned,
            edited_at: dto.edited_at,
            created_at: dto.created_at,
        }
    }
}

/// Member response
#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub guild_id: String,
    pub nickname: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
}

impl From<MemberDto> for MemberResponse {
    fn from(dto: MemberDto) -> Self {
        Self {
            user_id: dto.user_id,
            guild_id: dto.server_id,
            nickname: dto.nickname,
            roles: dto.roles,
            joined_at: dto.joined_at,
        }
    }
}

/// Message author (partial user)
#[derive(Debug, Serialize)]
pub struct MessageAuthor {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

/// Field-level validation error
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}
