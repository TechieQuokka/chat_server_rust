//! Application Services
//!
//! Business logic services that coordinate domain operations.
//!
//! ## Available Services
//!
//! - **AuthService**: Authentication, JWT tokens, password management
//! - **UserService**: User profile management
//! - **GuildService**: Server/guild management
//! - **ChannelService**: Channel operations
//! - **MessageService**: Message CRUD operations
//! - **RoleService**: Role management and member role assignments
//! - **InviteService**: Server invite management

pub mod auth_service;
pub mod user_service;
pub mod guild_service;
pub mod channel_service;
pub mod message_service;
pub mod role_service;
pub mod invite_service;

// Re-export auth service types
pub use auth_service::{AuthService, AuthServiceImpl, AuthTokens, AuthError, Claims};

// Re-export user service types
pub use user_service::{UserService, UserServiceImpl, UserDto, UpdateProfileDto, ServerPreviewDto, UserError};

// Re-export guild service types
pub use guild_service::{GuildService, GuildServiceImpl, GuildDto, CreateGuildDto, UpdateGuildDto, MemberDto, GuildError};

// Re-export channel service types
pub use channel_service::{ChannelService, ChannelServiceImpl, ChannelDto, CreateChannelDto, UpdateChannelDto, PermissionOverwriteDto, ChannelError};

// Re-export message service types
pub use message_service::{MessageService, MessageServiceImpl, MessageDto, CreateMessageDto, MessageQueryDto, MessageError};

// Re-export role service types
pub use role_service::{RoleService, RoleServiceImpl, RoleDto, CreateRoleDto, UpdateRoleDto, RolePositionDto, RoleError};

// Re-export invite service types
pub use invite_service::{
    InviteService, InviteServiceImpl, InviteDto, CreateInviteDto, InvitePreviewDto,
    InviteValidationDto, UseInviteResultDto, InviteError,
};
