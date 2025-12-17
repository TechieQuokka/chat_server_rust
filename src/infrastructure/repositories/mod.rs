//! Repository Implementations
//!
//! PostgreSQL implementations of domain repository traits.
//!
//! This module provides concrete implementations of the repository traits
//! defined in the domain layer. Each repository handles data access for
//! a specific entity type.
//!
//! ## Available Repositories
//!
//! - **UserRepository** - User account management
//! - **ServerRepository** - Server/guild operations (uses "servers" table)
//! - **ChannelRepository** - Channel management within guilds
//! - **RoleRepository** - Role management with permission bitfields
//! - **MemberRepository** - Server membership and role assignments
//! - **MessageRepository** - Message CRUD with cursor pagination
//! - **ReactionRepository** - Message reactions management
//! - **AttachmentRepository** - File attachment handling
//! - **InviteRepository** - Server invite links with expiration
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use sqlx::PgPool;
//! use crate::infrastructure::repositories::{
//!     PgUserRepository, PgServerRepository, PgChannelRepository,
//!     PgRoleRepository, PgMemberRepository, PgMessageRepository
//! };
//!
//! async fn setup_repositories(pool: PgPool) {
//!     let user_repo = PgUserRepository::new(pool.clone());
//!     let server_repo = PgServerRepository::new(pool.clone());
//!     let channel_repo = PgChannelRepository::new(pool.clone());
//!     let role_repo = PgRoleRepository::new(pool.clone());
//!     let member_repo = PgMemberRepository::new(pool.clone());
//!     let message_repo = PgMessageRepository::new(pool.clone());
//! }
//! ```

// Core repositories
pub mod user_repository;
pub mod server_repository;
pub mod channel_repository;
pub mod role_repository;
pub mod member_repository;
pub mod message_repository;

// Additional repositories
pub mod reaction_repository;
pub mod attachment_repository;
pub mod invite_repository;
pub mod session_repository;

// Keep guild_repository for backward compatibility during transition
#[deprecated(note = "Use server_repository instead - 'servers' is the actual table name")]
pub mod guild_repository;

// Re-export core repository structs for convenience
pub use user_repository::PgUserRepository;
pub use server_repository::PgServerRepository;
pub use channel_repository::PgChannelRepository;
pub use role_repository::PgRoleRepository;
pub use member_repository::PgMemberRepository;
pub use message_repository::PgMessageRepository;

// Re-export additional repository structs and traits
pub use reaction_repository::{
    MessageReaction, PgReactionRepository, ReactionGroup, ReactionRepository,
};
pub use attachment_repository::{
    AttachmentEntity, AttachmentRepository, CreateAttachment, PgAttachmentRepository,
};
pub use invite_repository::{
    CreateInvite, InviteEntity, InvitePreview, InviteRepository, PgInviteRepository,
};
pub use session_repository::PgSessionRepository;

// Backward compatibility - re-export old guild repository with deprecation warning
#[allow(deprecated)]
pub use guild_repository::PgGuildRepository;
