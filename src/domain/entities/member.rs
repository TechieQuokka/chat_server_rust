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

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Member Entity Tests
    // ==========================================================================

    fn create_test_member() -> Member {
        Member {
            server_id: 100,
            user_id: 200,
            nickname: None,
            joined_at: Utc::now(),
            roles: Vec::new(),
        }
    }

    #[test]
    fn test_member_default() {
        let member = Member::default();

        assert_eq!(member.server_id, 0);
        assert_eq!(member.user_id, 0);
        assert!(member.nickname.is_none());
        assert!(member.roles.is_empty());
    }

    #[test]
    fn test_member_new() {
        let member = Member::new(100, 200);

        assert_eq!(member.server_id, 100);
        assert_eq!(member.user_id, 200);
        assert!(member.nickname.is_none());
        assert!(member.roles.is_empty());
    }

    // ==========================================================================
    // has_role Tests
    // ==========================================================================

    #[test]
    fn test_member_has_role_true_when_present() {
        let mut member = create_test_member();
        member.roles = vec![101, 102, 103];

        assert!(member.has_role(101));
        assert!(member.has_role(102));
        assert!(member.has_role(103));
    }

    #[test]
    fn test_member_has_role_false_when_absent() {
        let mut member = create_test_member();
        member.roles = vec![101, 102];

        assert!(!member.has_role(103));
        assert!(!member.has_role(999));
    }

    #[test]
    fn test_member_has_role_empty_roles() {
        let member = create_test_member();

        assert!(!member.has_role(101));
    }

    // ==========================================================================
    // display_name Tests
    // ==========================================================================

    #[test]
    fn test_member_display_name_returns_nickname_when_set() {
        let mut member = create_test_member();
        member.nickname = Some("Cool Nickname".to_string());

        assert_eq!(member.display_name("username"), "Cool Nickname");
    }

    #[test]
    fn test_member_display_name_returns_username_when_no_nickname() {
        let member = create_test_member();

        assert_eq!(member.display_name("username"), "username");
    }

    #[test]
    fn test_member_display_name_returns_empty_nickname_when_set_to_empty() {
        let mut member = create_test_member();
        member.nickname = Some("".to_string());

        // When nickname is Some(""), returns the empty string
        assert_eq!(member.display_name("username"), "");
    }

    // ==========================================================================
    // Member Nickname Tests
    // ==========================================================================

    #[test]
    fn test_member_nickname_none_by_default() {
        let member = create_test_member();
        assert!(member.nickname.is_none());
    }

    #[test]
    fn test_member_nickname_can_be_set() {
        let mut member = create_test_member();
        member.nickname = Some("Server Nickname".to_string());

        assert_eq!(member.nickname, Some("Server Nickname".to_string()));
    }

    // ==========================================================================
    // Member Roles Tests
    // ==========================================================================

    #[test]
    fn test_member_roles_empty_by_default() {
        let member = create_test_member();
        assert!(member.roles.is_empty());
    }

    #[test]
    fn test_member_roles_can_be_added() {
        let mut member = create_test_member();
        member.roles.push(101);
        member.roles.push(102);

        assert_eq!(member.roles.len(), 2);
        assert!(member.has_role(101));
        assert!(member.has_role(102));
    }

    #[test]
    fn test_member_roles_can_be_removed() {
        let mut member = create_test_member();
        member.roles = vec![101, 102, 103];

        member.roles.retain(|&r| r != 102);

        assert_eq!(member.roles.len(), 2);
        assert!(member.has_role(101));
        assert!(!member.has_role(102));
        assert!(member.has_role(103));
    }

    // ==========================================================================
    // Member Clone Tests
    // ==========================================================================

    #[test]
    fn test_member_clone() {
        let mut member = create_test_member();
        member.nickname = Some("Nickname".to_string());
        member.roles = vec![101, 102];

        let cloned = member.clone();

        assert_eq!(member.server_id, cloned.server_id);
        assert_eq!(member.user_id, cloned.user_id);
        assert_eq!(member.nickname, cloned.nickname);
        assert_eq!(member.roles, cloned.roles);
    }

    // ==========================================================================
    // Member Serialization Tests
    // ==========================================================================

    #[test]
    fn test_member_serialization() {
        let mut member = create_test_member();
        member.nickname = Some("Test Nick".to_string());
        member.roles = vec![101, 102];

        let serialized = serde_json::to_string(&member).expect("Failed to serialize member");

        assert!(serialized.contains("\"server_id\":100"));
        assert!(serialized.contains("\"user_id\":200"));
        assert!(serialized.contains("\"nickname\":\"Test Nick\""));
        assert!(serialized.contains("\"roles\":[101,102]"));
    }

    #[test]
    fn test_member_serialization_empty_roles_default() {
        let member = create_test_member();

        let serialized = serde_json::to_string(&member).expect("Failed to serialize member");

        // roles should be an empty array
        assert!(serialized.contains("\"roles\":[]"));
    }

    // ==========================================================================
    // MemberRole Tests
    // ==========================================================================

    #[test]
    fn test_member_role_creation() {
        let member_role = MemberRole {
            server_id: 100,
            user_id: 200,
            role_id: 300,
        };

        assert_eq!(member_role.server_id, 100);
        assert_eq!(member_role.user_id, 200);
        assert_eq!(member_role.role_id, 300);
    }

    #[test]
    fn test_member_role_clone() {
        let member_role = MemberRole {
            server_id: 100,
            user_id: 200,
            role_id: 300,
        };

        let cloned = member_role.clone();

        assert_eq!(member_role.server_id, cloned.server_id);
        assert_eq!(member_role.user_id, cloned.user_id);
        assert_eq!(member_role.role_id, cloned.role_id);
    }

    // ==========================================================================
    // Member Composite Key Tests
    // ==========================================================================

    #[test]
    fn test_member_composite_key() {
        // Member is uniquely identified by (server_id, user_id)
        let member1 = Member::new(100, 200);
        let member2 = Member::new(100, 201); // Different user
        let member3 = Member::new(101, 200); // Different server

        // All three are distinct members
        assert_ne!((member1.server_id, member1.user_id), (member2.server_id, member2.user_id));
        assert_ne!((member1.server_id, member1.user_id), (member3.server_id, member3.user_id));
        assert_ne!((member2.server_id, member2.user_id), (member3.server_id, member3.user_id));
    }

    // ==========================================================================
    // Member Join Time Tests
    // ==========================================================================

    #[test]
    fn test_member_joined_at_is_set() {
        let before = Utc::now();
        let member = Member::new(100, 200);
        let after = Utc::now();

        assert!(member.joined_at >= before);
        assert!(member.joined_at <= after);
    }

    // ==========================================================================
    // Edge Cases
    // ==========================================================================

    #[test]
    fn test_member_with_many_roles() {
        let mut member = create_test_member();
        // Add 100 roles
        member.roles = (1..=100).collect();

        assert_eq!(member.roles.len(), 100);
        assert!(member.has_role(1));
        assert!(member.has_role(50));
        assert!(member.has_role(100));
        assert!(!member.has_role(101));
    }

    #[test]
    fn test_member_role_deduplication_not_enforced() {
        // Note: The Member struct doesn't enforce unique roles
        // This is a test to document current behavior
        let mut member = create_test_member();
        member.roles = vec![101, 101, 102]; // Duplicate role

        assert_eq!(member.roles.len(), 3); // Contains duplicate
    }

    #[test]
    fn test_member_display_name_with_special_characters() {
        let mut member = create_test_member();
        member.nickname = Some("Test <script>alert('xss')</script>".to_string());

        // The nickname is stored as-is; escaping is the responsibility of display layer
        assert_eq!(
            member.display_name("username"),
            "Test <script>alert('xss')</script>"
        );
    }

    #[test]
    fn test_member_display_name_with_unicode() {
        let mut member = create_test_member();
        member.nickname = Some("Test User".to_string());

        assert_eq!(member.display_name("username"), "Test User");
    }
}
