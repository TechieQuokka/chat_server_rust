//! Server (Guild) entity and repository trait.
//!
//! Maps to the `servers` table in the database schema.
//! Note: The entity is named `Server` to match the database table,
//! but re-exported as `Guild` for API compatibility.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Represents a server (guild) in the chat system.
///
/// A server is a community space containing channels, roles, and members.
///
/// Maps to the `servers` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - name: VARCHAR(100) NOT NULL
/// - owner_id: BIGINT NOT NULL REFERENCES users(id)
/// - icon_url: TEXT NULL
/// - description: TEXT NULL
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - updated_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Server name (1-100 characters)
    pub name: String,

    /// User ID of the server owner
    pub owner_id: i64,

    /// URL to server icon image
    pub icon_url: Option<String>,

    /// Server description
    pub description: Option<String>,

    /// Server creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Server {
    /// Check if a user is the owner of this server.
    pub fn is_owner(&self, user_id: i64) -> bool {
        self.owner_id == user_id
    }
}

impl Default for Server {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            name: String::new(),
            owner_id: 0,
            icon_url: None,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Type alias for API compatibility.
/// In Discord terminology, servers are called "guilds".
pub type Guild = Server;

/// Repository trait for Server data access operations.
#[async_trait]
pub trait ServerRepository: Send + Sync {
    /// Find a server by its Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Server>, AppError>;

    /// Find all servers a user is a member of.
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Server>, AppError>;

    /// Find all servers owned by a user.
    async fn find_by_owner_id(&self, owner_id: i64) -> Result<Vec<Server>, AppError>;

    /// Create a new server.
    async fn create(&self, server: &Server) -> Result<Server, AppError>;

    /// Update an existing server.
    async fn update(&self, server: &Server) -> Result<Server, AppError>;

    /// Delete a server (cascading delete of channels, roles, members).
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Get the member count for a server.
    async fn get_member_count(&self, id: i64) -> Result<i64, AppError>;

    /// Transfer ownership to another user.
    async fn transfer_ownership(&self, server_id: i64, new_owner_id: i64) -> Result<(), AppError>;
}

/// Type alias for API compatibility.
pub type GuildRepository = dyn ServerRepository;
