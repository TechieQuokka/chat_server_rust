//! Server Invite entity and repository trait.
//!
//! Maps to the `invites` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Represents a server invite link.
///
/// Maps to the `invites` table:
/// - code: VARCHAR(10) PRIMARY KEY
/// - server_id: BIGINT NOT NULL REFERENCES servers(id)
/// - channel_id: BIGINT NOT NULL REFERENCES channels(id)
/// - inviter_id: BIGINT REFERENCES users(id) -- NULL if inviter deleted
/// - max_uses: INTEGER NOT NULL DEFAULT 0 (0 = unlimited)
/// - uses: INTEGER NOT NULL DEFAULT 0
/// - max_age: INTEGER NOT NULL DEFAULT 0 (seconds, 0 = never expires)
/// - temporary: BOOLEAN NOT NULL DEFAULT FALSE
/// - expires_at: TIMESTAMPTZ NULL (computed from max_age)
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    /// Short invite code (e.g., "aBcD1234"), primary key
    pub code: String,

    /// Server ID this invite is for
    pub server_id: i64,

    /// Channel ID the invite leads to
    pub channel_id: i64,

    /// User ID who created the invite (None if user deleted)
    pub inviter_id: Option<i64>,

    /// Maximum number of uses (0 = unlimited)
    pub max_uses: i32,

    /// Current number of times this invite was used
    pub uses: i32,

    /// Seconds until expiration (0 = never expires)
    pub max_age: i32,

    /// Whether members gain temporary membership (kicked when offline)
    pub temporary: bool,

    /// Pre-computed expiration timestamp (None if never expires)
    pub expires_at: Option<DateTime<Utc>>,

    /// When the invite was created
    pub created_at: DateTime<Utc>,
}

impl Invite {
    /// Check if the invite has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if the invite has reached its maximum uses.
    pub fn is_maxed_out(&self) -> bool {
        self.max_uses > 0 && self.uses >= self.max_uses
    }

    /// Check if the invite is still valid (not expired and not maxed out).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_maxed_out()
    }

    /// Get remaining uses (None if unlimited).
    pub fn remaining_uses(&self) -> Option<i32> {
        if self.max_uses > 0 {
            Some(self.max_uses - self.uses)
        } else {
            None
        }
    }

    /// Generate a random invite code.
    pub fn generate_code() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const CODE_LEN: usize = 8;

        let mut rng = rand::thread_rng();
        (0..CODE_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Create a new invite with the given parameters.
    pub fn new(
        server_id: i64,
        channel_id: i64,
        inviter_id: i64,
        max_uses: i32,
        max_age: i32,
        temporary: bool,
    ) -> Self {
        let now = Utc::now();
        let expires_at = if max_age > 0 {
            Some(now + chrono::Duration::seconds(max_age as i64))
        } else {
            None
        };

        Self {
            code: Self::generate_code(),
            server_id,
            channel_id,
            inviter_id: Some(inviter_id),
            max_uses,
            uses: 0,
            max_age,
            temporary,
            expires_at,
            created_at: now,
        }
    }
}

impl Default for Invite {
    fn default() -> Self {
        Self {
            code: Self::generate_code(),
            server_id: 0,
            channel_id: 0,
            inviter_id: None,
            max_uses: 0,
            uses: 0,
            max_age: 0,
            temporary: false,
            expires_at: None,
            created_at: Utc::now(),
        }
    }
}

/// Repository trait for Invite data access operations.
#[async_trait]
pub trait InviteRepository: Send + Sync {
    /// Find an invite by its code.
    async fn find_by_code(&self, code: &str) -> Result<Option<Invite>, AppError>;

    /// Find all invites for a server.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<Invite>, AppError>;

    /// Find all invites for a channel.
    async fn find_by_channel_id(&self, channel_id: i64) -> Result<Vec<Invite>, AppError>;

    /// Find all invites created by a user.
    async fn find_by_inviter_id(&self, inviter_id: i64) -> Result<Vec<Invite>, AppError>;

    /// Create a new invite.
    async fn create(&self, invite: &Invite) -> Result<Invite, AppError>;

    /// Delete an invite by code.
    async fn delete(&self, code: &str) -> Result<(), AppError>;

    /// Increment the uses count for an invite.
    async fn increment_uses(&self, code: &str) -> Result<(), AppError>;

    /// Delete all expired invites (cleanup job).
    async fn delete_expired(&self) -> Result<i64, AppError>;

    /// Check if an invite code exists.
    async fn code_exists(&self, code: &str) -> Result<bool, AppError>;
}
