//! User entity and repository trait.
//!
//! Maps to the `users` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// User status enum matching database VARCHAR constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    #[default]
    Offline,
    Online,
    Idle,
    Dnd,
    Invisible,
}

impl UserStatus {
    /// Convert from database string representation.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "online" => Self::Online,
            "idle" => Self::Idle,
            "dnd" => Self::Dnd,
            "invisible" => Self::Invisible,
            _ => Self::Offline,
        }
    }

    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Online => "online",
            Self::Idle => "idle",
            Self::Dnd => "dnd",
            Self::Invisible => "invisible",
        }
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a user account in the chat system.
///
/// Maps to the `users` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - username: VARCHAR(32) NOT NULL UNIQUE
/// - email: VARCHAR(255) NOT NULL UNIQUE
/// - password_hash: VARCHAR(255) NOT NULL
/// - display_name: VARCHAR(32) NULL
/// - avatar_url: TEXT NULL
/// - status: VARCHAR(20) DEFAULT 'offline'
/// - bio: TEXT NULL
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - updated_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Username (2-32 characters, unique)
    pub username: String,

    /// Email address (unique)
    pub email: String,

    /// Argon2 password hash
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// Display name (optional, up to 32 characters)
    pub display_name: Option<String>,

    /// URL to user's avatar image
    pub avatar_url: Option<String>,

    /// User's online status
    #[serde(default)]
    pub status: UserStatus,

    /// User's bio/about me text
    pub bio: Option<String>,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Get the user's display name, falling back to username if not set.
    pub fn display_name_or_username(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Check if the user is currently online (online, idle, or dnd).
    pub fn is_online(&self) -> bool {
        matches!(self.status, UserStatus::Online | UserStatus::Idle | UserStatus::Dnd)
    }
}

impl Default for User {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            username: String::new(),
            email: String::new(),
            password_hash: String::new(),
            display_name: None,
            avatar_url: None,
            status: UserStatus::default(),
            bio: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Repository trait for User data access operations.
///
/// Implementations of this trait handle the actual database interactions.
/// The trait is defined in the domain layer to maintain dependency inversion.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find a user by their Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError>;

    /// Find a user by their email address.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;

    /// Find a user by username.
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;

    /// Create a new user in the database.
    async fn create(&self, user: &User) -> Result<User, AppError>;

    /// Update an existing user.
    async fn update(&self, user: &User) -> Result<User, AppError>;

    /// Delete a user by ID.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Check if an email address is already registered.
    async fn email_exists(&self, email: &str) -> Result<bool, AppError>;

    /// Check if a username is already taken.
    async fn username_exists(&self, username: &str) -> Result<bool, AppError>;

    /// Update user's online status.
    async fn update_status(&self, id: i64, status: UserStatus) -> Result<(), AppError>;
}
