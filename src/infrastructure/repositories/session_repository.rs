//! Session Repository Implementation
//!
//! PostgreSQL implementation of the SessionRepository trait.
//! Handles user sessions for JWT refresh token management.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::{DeviceType, Session, SessionRepository};
use crate::shared::error::AppError;

/// Database row representation matching the user_sessions table schema.
/// Note: ip_address is stored as String because sqlx doesn't support std::net::IpAddr
/// for PostgreSQL INET type directly.
#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: i64,
    refresh_token_hash: String,
    device_info: Option<String>,
    device_type: Option<String>,
    os_info: Option<String>,
    ip_address: Option<String>, // PostgreSQL INET stored as String
    location_info: Option<serde_json::Value>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl SessionRow {
    /// Convert database row to domain Session entity.
    fn into_session(self) -> Session {
        Session {
            id: self.id,
            user_id: self.user_id,
            refresh_token_hash: self.refresh_token_hash,
            device_info: self.device_info,
            device_type: self.device_type.map(|s| DeviceType::from_str(&s)),
            os_info: self.os_info,
            ip_address: self.ip_address.and_then(|s| s.parse::<IpAddr>().ok()),
            location_info: self.location_info,
            expires_at: self.expires_at,
            created_at: self.created_at,
            last_used_at: self.last_used_at,
            revoked_at: self.revoked_at,
        }
    }
}

/// PostgreSQL session repository implementation.
#[derive(Clone)]
pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    /// Create a new PgSessionRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    /// Find a session by its UUID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, AppError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, refresh_token_hash, device_info, device_type, os_info,
                   ip_address, location_info, expires_at, created_at, last_used_at, revoked_at
            FROM user_sessions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_session()))
    }

    /// Find a session by refresh token hash.
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AppError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, refresh_token_hash, device_info, device_type, os_info,
                   ip_address, location_info, expires_at, created_at, last_used_at, revoked_at
            FROM user_sessions
            WHERE refresh_token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_session()))
    }

    /// Find all active sessions for a user.
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Session>, AppError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, refresh_token_hash, device_info, device_type, os_info,
                   ip_address, location_info, expires_at, created_at, last_used_at, revoked_at
            FROM user_sessions
            WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
            ORDER BY last_used_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_session()).collect())
    }

    /// Create a new session.
    async fn create(&self, session: &Session) -> Result<Session, AppError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            INSERT INTO user_sessions (
                id, user_id, refresh_token_hash, device_info, device_type, os_info,
                ip_address, location_info, expires_at, created_at, last_used_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, user_id, refresh_token_hash, device_info, device_type, os_info,
                      ip_address, location_info, expires_at, created_at, last_used_at, revoked_at
            "#,
        )
        .bind(session.id)
        .bind(session.user_id)
        .bind(&session.refresh_token_hash)
        .bind(&session.device_info)
        .bind(session.device_type.as_ref().map(|d| d.as_str()))
        .bind(&session.os_info)
        .bind(session.ip_address.map(|ip| ip.to_string()))
        .bind(&session.location_info)
        .bind(session.expires_at)
        .bind(session.created_at)
        .bind(session.last_used_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into_session())
    }

    /// Update last_used_at timestamp.
    async fn touch(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE user_sessions SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Revoke a session (set revoked_at).
    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE user_sessions SET revoked_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Revoke all sessions for a user, optionally keeping one.
    async fn revoke_all_for_user(
        &self,
        user_id: i64,
        except_session_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let result = if let Some(except_id) = except_session_id {
            sqlx::query(
                r#"
                UPDATE user_sessions
                SET revoked_at = NOW()
                WHERE user_id = $1 AND revoked_at IS NULL AND id != $2
                "#,
            )
            .bind(user_id)
            .bind(except_id)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                UPDATE user_sessions
                SET revoked_at = NOW()
                WHERE user_id = $1 AND revoked_at IS NULL
                "#,
            )
            .bind(user_id)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected() as i64)
    }

    /// Delete a session.
    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete all expired and old revoked sessions (cleanup job).
    async fn cleanup_expired(&self) -> Result<i64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_sessions
            WHERE expires_at < NOW()
               OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '7 days')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Get count of active sessions for a user.
    async fn count_active(&self, user_id: i64) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM user_sessions
            WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Find sessions by IP address (for security monitoring).
    async fn find_by_ip(&self, ip_address: IpAddr) -> Result<Vec<Session>, AppError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, refresh_token_hash, device_info, device_type, os_info,
                   ip_address, location_info, expires_at, created_at, last_used_at, revoked_at
            FROM user_sessions
            WHERE ip_address = $1
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .bind(ip_address.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_session()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
}
