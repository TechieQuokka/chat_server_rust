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

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // UserStatus Tests
    // ==========================================================================

    #[test]
    fn test_user_status_default_is_offline() {
        let status = UserStatus::default();
        assert_eq!(status, UserStatus::Offline);
    }

    #[test]
    fn test_user_status_from_str_online() {
        assert_eq!(UserStatus::from_str("online"), UserStatus::Online);
        assert_eq!(UserStatus::from_str("ONLINE"), UserStatus::Online);
        assert_eq!(UserStatus::from_str("Online"), UserStatus::Online);
    }

    #[test]
    fn test_user_status_from_str_offline() {
        assert_eq!(UserStatus::from_str("offline"), UserStatus::Offline);
        assert_eq!(UserStatus::from_str("OFFLINE"), UserStatus::Offline);
    }

    #[test]
    fn test_user_status_from_str_idle() {
        assert_eq!(UserStatus::from_str("idle"), UserStatus::Idle);
        assert_eq!(UserStatus::from_str("IDLE"), UserStatus::Idle);
    }

    #[test]
    fn test_user_status_from_str_dnd() {
        assert_eq!(UserStatus::from_str("dnd"), UserStatus::Dnd);
        assert_eq!(UserStatus::from_str("DND"), UserStatus::Dnd);
    }

    #[test]
    fn test_user_status_from_str_invisible() {
        assert_eq!(UserStatus::from_str("invisible"), UserStatus::Invisible);
        assert_eq!(UserStatus::from_str("INVISIBLE"), UserStatus::Invisible);
    }

    #[test]
    fn test_user_status_from_str_unknown_defaults_to_offline() {
        assert_eq!(UserStatus::from_str("unknown"), UserStatus::Offline);
        assert_eq!(UserStatus::from_str(""), UserStatus::Offline);
        assert_eq!(UserStatus::from_str("invalid"), UserStatus::Offline);
    }

    #[test]
    fn test_user_status_as_str_roundtrip() {
        let statuses = vec![
            UserStatus::Offline,
            UserStatus::Online,
            UserStatus::Idle,
            UserStatus::Dnd,
            UserStatus::Invisible,
        ];

        for status in statuses {
            let s = status.as_str();
            let parsed = UserStatus::from_str(s);
            assert_eq!(parsed, status, "Roundtrip failed for {:?}", status);
        }
    }

    #[test]
    fn test_user_status_as_str_values() {
        assert_eq!(UserStatus::Offline.as_str(), "offline");
        assert_eq!(UserStatus::Online.as_str(), "online");
        assert_eq!(UserStatus::Idle.as_str(), "idle");
        assert_eq!(UserStatus::Dnd.as_str(), "dnd");
        assert_eq!(UserStatus::Invisible.as_str(), "invisible");
    }

    #[test]
    fn test_user_status_display() {
        assert_eq!(format!("{}", UserStatus::Online), "online");
        assert_eq!(format!("{}", UserStatus::Offline), "offline");
        assert_eq!(format!("{}", UserStatus::Idle), "idle");
        assert_eq!(format!("{}", UserStatus::Dnd), "dnd");
        assert_eq!(format!("{}", UserStatus::Invisible), "invisible");
    }

    // ==========================================================================
    // User Entity Tests
    // ==========================================================================

    fn create_test_user() -> User {
        User {
            id: 12345678901234567,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            display_name: None,
            avatar_url: None,
            status: UserStatus::Offline,
            bio: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_user_default() {
        let user = User::default();

        assert_eq!(user.id, 0);
        assert!(user.username.is_empty());
        assert!(user.email.is_empty());
        assert!(user.password_hash.is_empty());
        assert!(user.display_name.is_none());
        assert!(user.avatar_url.is_none());
        assert_eq!(user.status, UserStatus::Offline);
        assert!(user.bio.is_none());
    }

    #[test]
    fn test_user_display_name_or_username_returns_display_name_when_set() {
        let mut user = create_test_user();
        user.display_name = Some("Display Name".to_string());

        assert_eq!(user.display_name_or_username(), "Display Name");
    }

    #[test]
    fn test_user_display_name_or_username_returns_username_when_none() {
        let user = create_test_user();
        assert!(user.display_name.is_none());

        assert_eq!(user.display_name_or_username(), "testuser");
    }

    #[test]
    fn test_user_display_name_or_username_returns_username_when_empty() {
        let mut user = create_test_user();
        user.display_name = Some("".to_string());

        // When display_name is Some(""), it returns the empty string
        assert_eq!(user.display_name_or_username(), "");
    }

    // ==========================================================================
    // User Online Status Tests
    // ==========================================================================

    #[test]
    fn test_user_is_online_true_for_online_status() {
        let mut user = create_test_user();
        user.status = UserStatus::Online;

        assert!(user.is_online());
    }

    #[test]
    fn test_user_is_online_true_for_idle_status() {
        let mut user = create_test_user();
        user.status = UserStatus::Idle;

        assert!(user.is_online());
    }

    #[test]
    fn test_user_is_online_true_for_dnd_status() {
        let mut user = create_test_user();
        user.status = UserStatus::Dnd;

        assert!(user.is_online());
    }

    #[test]
    fn test_user_is_online_false_for_offline_status() {
        let mut user = create_test_user();
        user.status = UserStatus::Offline;

        assert!(!user.is_online());
    }

    #[test]
    fn test_user_is_online_false_for_invisible_status() {
        let mut user = create_test_user();
        user.status = UserStatus::Invisible;

        assert!(!user.is_online());
    }

    // ==========================================================================
    // User Serialization Tests
    // ==========================================================================

    #[test]
    fn test_user_password_hash_not_serialized() {
        let user = create_test_user();

        let serialized = serde_json::to_string(&user).expect("Failed to serialize user");

        // password_hash should not appear in serialized output
        assert!(!serialized.contains("password_hash"));
        assert!(!serialized.contains("hashed_password"));
    }

    #[test]
    fn test_user_serialization_includes_required_fields() {
        let user = create_test_user();

        let serialized = serde_json::to_string(&user).expect("Failed to serialize user");

        assert!(serialized.contains("\"id\":12345678901234567"));
        assert!(serialized.contains("\"username\":\"testuser\""));
        assert!(serialized.contains("\"email\":\"test@example.com\""));
    }

    #[test]
    fn test_user_status_serializes_lowercase() {
        let mut user = create_test_user();
        user.status = UserStatus::Online;

        let serialized = serde_json::to_string(&user).expect("Failed to serialize user");

        assert!(serialized.contains("\"status\":\"online\""));
    }

    // ==========================================================================
    // User Clone Tests
    // ==========================================================================

    #[test]
    fn test_user_clone() {
        let user = create_test_user();
        let cloned = user.clone();

        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.status, cloned.status);
    }

    // ==========================================================================
    // UserStatus Equality Tests
    // ==========================================================================

    #[test]
    fn test_user_status_equality() {
        assert_eq!(UserStatus::Online, UserStatus::Online);
        assert_ne!(UserStatus::Online, UserStatus::Offline);
    }

    #[test]
    fn test_user_status_clone() {
        let status = UserStatus::Online;
        let cloned = status.clone();

        assert_eq!(status, cloned);
    }
}
