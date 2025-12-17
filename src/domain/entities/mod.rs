//! # Domain Entities
//!
//! Core domain entities representing the main business objects in the chat server.
//! All entities map directly to their corresponding database tables.
//!
//! ## Core Entities
//!
//! - **User**: User account with authentication data and profile
//! - **Server/Guild**: A server/community that contains channels and members
//! - **Channel**: A communication space within a server (or DM)
//! - **Message**: A text message sent in a channel
//! - **Role**: A set of permissions assignable to server members
//! - **Member**: A user's membership in a specific server
//!
//! ## Supporting Entities
//!
//! - **Invite**: Server invite links with usage tracking
//! - **Attachment**: File attachments on messages
//! - **Reaction**: Emoji reactions on messages
//! - **Session**: User sessions for JWT refresh token management
//!
//! ## Repository Traits
//!
//! Each entity has an associated repository trait defining data access operations.
//! These traits are implemented in the infrastructure layer, following the
//! dependency inversion principle.

mod user;
mod guild;
mod channel;
mod message;
mod role;
mod member;
mod invite;
mod attachment;
mod reaction;
mod session;

// Re-export User entity and related types
pub use user::{User, UserStatus, UserRepository};

// Re-export Server/Guild entity and related types
// Note: Server is the database table name, Guild is the API terminology
pub use guild::{Server, Guild, ServerRepository, GuildRepository};

// Re-export Channel entity and related types
pub use channel::{Channel, ChannelType, PermissionOverwrite, ChannelRepository};

// Re-export Message entity and related types
pub use message::{Message, MessageType, MessageRepository};

// Re-export Role entity and related types
pub use role::{Role, RoleRepository, permissions};

// Re-export Member entity and related types
pub use member::{Member, MemberRole, MemberRepository};

// Re-export Invite entity and related types
pub use invite::{Invite, InviteRepository};

// Re-export Attachment entity and related types
pub use attachment::{Attachment, AttachmentRepository, MAX_ATTACHMENT_SIZE};

// Re-export Reaction entity and related types
pub use reaction::{Reaction, ReactionCount, ReactionRepository};

// Re-export Session entity and related types
pub use session::{Session, DeviceType, SessionRepository};
