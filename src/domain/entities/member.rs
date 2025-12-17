//! Server Member entity and repository trait.
//!
//! Maps to the `server_members` and `member_roles` tables in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Represents a user's membership in a server.
///
/// Maps to the `server_members` table:
/// - server_id: BIGINT NOT NULL REFERENCES servers(id) (composite PK)
/// - user_id: BIGINT NOT NULL REFERENCES users(id) (composite PK)
/// - nickname: VARCHAR(32) NULL
/// - joined_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
///
/// Role assignments are stored in the `member_roles` junction table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    /// Server ID (part of composite primary key)
    pub server_id: i64,

    /// User ID (part of composite primary key)
    pub user_id: i64,

    /// Server-specific nickname (if different from username)
    pub nickname: Option<String>,

    /// When the user joined the server
    pub joined_at: DateTime<Utc>,

    /// IDs of roles assigned to this member (loaded from member_roles table)
    #[serde(default)]
    pub roles: Vec<i64>,
}

impl Member {
    /// Check if the member has a specific role.
    pub fn has_role(&self, role_id: i64) -> bool {
        self.roles.contains(&role_id)
    }

    /// Get the display name (nickname or fallback to provided username).
    pub fn display_name<'a>(&'a self, username: &'a str) -> &'a str {
        self.nickname.as_deref().unwrap_or(username)
    }

    /// Create a new member with just the required fields.
    pub fn new(server_id: i64, user_id: i64) -> Self {
        Self {
            server_id,
            user_id,
            nickname: None,
            joined_at: Utc::now(),
            roles: Vec::new(),
        }
    }
}

impl Default for Member {
    fn default() -> Self {
        Self {
            server_id: 0,
            user_id: 0,
            nickname: None,
            joined_at: Utc::now(),
            roles: Vec::new(),
        }
    }
}

/// Represents a role assignment for a member.
///
/// Maps to the `member_roles` table:
/// - server_id: BIGINT NOT NULL (composite PK)
/// - user_id: BIGINT NOT NULL (composite PK)
/// - role_id: BIGINT NOT NULL REFERENCES roles(id) (composite PK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRole {
    /// Server ID
    pub server_id: i64,

    /// User ID
    pub user_id: i64,

    /// Role ID
    pub role_id: i64,
}

/// Repository trait for Member data access operations.
#[async_trait]
pub trait MemberRepository: Send + Sync {
    /// Find a member by server and user ID.
    async fn find(&self, server_id: i64, user_id: i64) -> Result<Option<Member>, AppError>;

    /// Find all servers a user is a member of.
    async fn find_by_user(&self, user_id: i64) -> Result<Vec<Member>, AppError>;

    /// Find all members in a server with cursor-based pagination.
    async fn find_by_server_id(
        &self,
        server_id: i64,
        after: Option<i64>,
        limit: i32,
    ) -> Result<Vec<Member>, AppError>;

    /// Search members by nickname or username.
    async fn search(
        &self,
        server_id: i64,
        query: &str,
        limit: i32,
    ) -> Result<Vec<Member>, AppError>;

    /// Add a member to a server.
    async fn create(&self, member: &Member) -> Result<Member, AppError>;

    /// Update a member (nickname).
    async fn update(&self, member: &Member) -> Result<Member, AppError>;

    /// Remove a member from a server.
    async fn delete(&self, server_id: i64, user_id: i64) -> Result<(), AppError>;

    /// Add a role to a member.
    async fn add_role(&self, server_id: i64, user_id: i64, role_id: i64) -> Result<(), AppError>;

    /// Remove a role from a member.
    async fn remove_role(&self, server_id: i64, user_id: i64, role_id: i64) -> Result<(), AppError>;

    /// Get all role IDs for a member.
    async fn get_roles(&self, server_id: i64, user_id: i64) -> Result<Vec<i64>, AppError>;

    /// Check if a user is a member of a server.
    async fn is_member(&self, server_id: i64, user_id: i64) -> Result<bool, AppError>;

    /// Get the member count for a server.
    async fn count_by_server(&self, server_id: i64) -> Result<i64, AppError>;

    /// Find members with a specific role.
    async fn find_by_role(&self, server_id: i64, role_id: i64) -> Result<Vec<Member>, AppError>;
}
