//! User Session entity and repository trait.
//!
//! Maps to the `user_sessions` table in the database schema.
//! Used for JWT refresh token management and device tracking.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

use crate::shared::error::AppError;

/// Device type for sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Browser,
    Bot,
    #[default]
    Unknown,
}

impl DeviceType {
    /// Convert from database string representation.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desktop" => Self::Desktop,
            "mobile" => Self::Mobile,
            "tablet" => Self::Tablet,
            "browser" => Self::Browser,
            "bot" => Self::Bot,
            _ => Self::Unknown,
        }
    }

    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Tablet => "tablet",
            Self::Browser => "browser",
            Self::Bot => "bot",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a user session for JWT refresh token management.
///
/// Maps to the `user_sessions` table:
/// - id: UUID PRIMARY KEY DEFAULT gen_random_uuid()
/// - user_id: BIGINT NOT NULL REFERENCES users(id)
/// - refresh_token_hash: VARCHAR(255) NOT NULL (SHA-256 hash)
/// - device_info: TEXT NULL (user agent string)
/// - device_type: VARCHAR(20) NULL
/// - os_info: VARCHAR(50) NULL
/// - ip_address: INET NULL
/// - location_info: JSONB NULL
/// - expires_at: TIMESTAMPTZ NOT NULL
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - last_used_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// - revoked_at: TIMESTAMPTZ NULL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// UUID primary key
    pub id: Uuid,

    /// User ID this session belongs to
    pub user_id: i64,

    /// SHA-256 hash of the refresh token (never store raw tokens)
    #[serde(skip_serializing)]
    pub refresh_token_hash: String,

    /// Raw user agent string or device description
    pub device_info: Option<String>,

    /// Normalized device category
    pub device_type: Option<DeviceType>,

    /// Operating system info
    pub os_info: Option<String>,

    /// Client IP address at session creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<IpAddr>,

    /// Geo-location data if available (JSON)
    pub location_info: Option<serde_json::Value>,

    /// When this session expires
    pub expires_at: DateTime<Utc>,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// When the session was last used (refresh token used)
    pub last_used_at: DateTime<Utc>,

    /// When the session was revoked (None if active)
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    /// Check if the session is currently active (not expired, not revoked).
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > Utc::now()
    }

    /// Check if the session has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Get a display name for this session (device + OS).
    pub fn display_name(&self) -> String {
        let device = self.device_type
            .as_ref()
            .map(|d| d.as_str())
            .unwrap_or("Unknown device");

        let os = self.os_info
            .as_deref()
            .unwrap_or("Unknown OS");

        format!("{} on {}", device, os)
    }

    /// Create a new session.
    pub fn new(
        user_id: i64,
        refresh_token_hash: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            refresh_token_hash,
            device_info: None,
            device_type: None,
            os_info: None,
            ip_address: None,
            location_info: None,
            expires_at,
            created_at: now,
            last_used_at: now,
            revoked_at: None,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: 0,
            refresh_token_hash: String::new(),
            device_info: None,
            device_type: None,
            os_info: None,
            ip_address: None,
            location_info: None,
            expires_at: now,
            created_at: now,
            last_used_at: now,
            revoked_at: None,
        }
    }
}

/// Repository trait for Session data access operations.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Find a session by its UUID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, AppError>;

    /// Find a session by refresh token hash.
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AppError>;

    /// Find all active sessions for a user.
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Session>, AppError>;

    /// Create a new session.
    async fn create(&self, session: &Session) -> Result<Session, AppError>;

    /// Update last_used_at timestamp.
    async fn touch(&self, id: Uuid) -> Result<(), AppError>;

    /// Revoke a session (set revoked_at).
    async fn revoke(&self, id: Uuid) -> Result<(), AppError>;

    /// Revoke all sessions for a user, optionally keeping one.
    async fn revoke_all_for_user(
        &self,
        user_id: i64,
        except_session_id: Option<Uuid>,
    ) -> Result<i64, AppError>;

    /// Delete a session.
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    /// Delete all expired and old revoked sessions (cleanup job).
    async fn cleanup_expired(&self) -> Result<i64, AppError>;

    /// Get count of active sessions for a user.
    async fn count_active(&self, user_id: i64) -> Result<i64, AppError>;

    /// Find sessions by IP address (for security monitoring).
    async fn find_by_ip(&self, ip_address: IpAddr) -> Result<Vec<Session>, AppError>;
}
