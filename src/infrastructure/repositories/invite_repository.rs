//! Invite Repository Implementation
//!
//! PostgreSQL implementation of server invite operations.
//! Handles invite code generation, validation, and usage tracking.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::shared::error::AppError;

/// Invite entity representing a server invite link.
///
/// Invites can be limited by usage count and/or expiration time.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InviteEntity {
    /// Short alphanumeric invite code (e.g., "aBcD1234")
    pub code: String,
    /// ID of the server this invite is for
    pub server_id: i64,
    /// ID of the channel the invite leads to
    pub channel_id: i64,
    /// ID of the user who created the invite (None if deleted)
    pub inviter_id: Option<i64>,
    /// Maximum number of uses (0 = unlimited)
    pub max_uses: i32,
    /// Current number of times this invite was used
    pub uses: i32,
    /// Seconds until expiration (0 = never)
    pub max_age: i32,
    /// If true, members are kicked when they go offline
    pub temporary: bool,
    /// Pre-computed expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// When the invite was created
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new invite.
#[derive(Debug, Clone)]
pub struct CreateInvite {
    pub code: String,
    pub server_id: i64,
    pub channel_id: i64,
    pub inviter_id: Option<i64>,
    pub max_uses: i32,
    pub max_age: i32,
    pub temporary: bool,
}

/// Trait defining invite repository operations.
#[async_trait]
pub trait InviteRepository: Send + Sync {
    /// Find an invite by its code.
    async fn find_by_code(&self, code: &str) -> Result<Option<InviteEntity>, AppError>;

    /// Find all invites for a server.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<InviteEntity>, AppError>;

    /// Create a new invite.
    async fn create(&self, invite: &CreateInvite) -> Result<InviteEntity, AppError>;

    /// Delete an invite by code.
    async fn delete(&self, code: &str) -> Result<(), AppError>;

    /// Increment the usage count of an invite.
    ///
    /// Returns an error if the invite is at max uses.
    async fn increment_uses(&self, code: &str) -> Result<(), AppError>;

    /// Check if an invite is valid (not expired and not maxed out).
    async fn is_valid(&self, code: &str) -> Result<bool, AppError>;
}

/// PostgreSQL implementation of the InviteRepository.
pub struct PgInviteRepository {
    pool: PgPool,
}

impl PgInviteRepository {
    /// Creates a new PgInviteRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InviteRepository for PgInviteRepository {
    /// Find an invite by its code.
    ///
    /// Returns None if the invite does not exist.
    async fn find_by_code(&self, code: &str) -> Result<Option<InviteEntity>, AppError> {
        let invite = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE code = $1
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(invite)
    }

    /// Find all invites for a server.
    ///
    /// Returns invites ordered by creation time (newest first).
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<InviteEntity>, AppError> {
        let invites = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE server_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(invites)
    }

    /// Create a new invite.
    ///
    /// The expiration time is computed from max_age if provided.
    async fn create(&self, invite: &CreateInvite) -> Result<InviteEntity, AppError> {
        // Compute expires_at from max_age (0 means never expires)
        let expires_at = if invite.max_age > 0 {
            Some(Utc::now() + chrono::Duration::seconds(invite.max_age as i64))
        } else {
            None
        };

        let created = sqlx::query_as::<_, InviteEntity>(
            r#"
            INSERT INTO invites (code, server_id, channel_id, inviter_id, max_uses, max_age, temporary, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING code, server_id, channel_id, inviter_id, max_uses, uses,
                      max_age, temporary, expires_at, created_at
            "#,
        )
        .bind(&invite.code)
        .bind(invite.server_id)
        .bind(invite.channel_id)
        .bind(invite.inviter_id)
        .bind(invite.max_uses)
        .bind(invite.max_age)
        .bind(invite.temporary)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(created)
    }

    /// Delete an invite by code.
    ///
    /// Returns an error if the invite does not exist.
    async fn delete(&self, code: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM invites WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Invite {} not found", code)));
        }

        Ok(())
    }

    /// Increment the usage count of an invite.
    ///
    /// Uses a conditional update that only succeeds if the invite
    /// is valid (not expired, not at max uses).
    async fn increment_uses(&self, code: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE invites
            SET uses = uses + 1
            WHERE code = $1
              AND (expires_at IS NULL OR expires_at > NOW())
              AND (max_uses = 0 OR uses < max_uses)
            "#,
        )
        .bind(code)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            // Either invite doesn't exist or is no longer valid
            let invite = self.find_by_code(code).await?;
            match invite {
                None => return Err(AppError::NotFound(format!("Invite {} not found", code))),
                Some(inv) => {
                    if inv.expires_at.map(|e| e <= Utc::now()).unwrap_or(false) {
                        return Err(AppError::BadRequest("Invite has expired".to_string()));
                    }
                    if inv.max_uses > 0 && inv.uses >= inv.max_uses {
                        return Err(AppError::BadRequest(
                            "Invite has reached maximum uses".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if an invite is valid (not expired and not maxed out).
    ///
    /// Returns false for non-existent invites.
    async fn is_valid(&self, code: &str) -> Result<bool, AppError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM invites
                WHERE code = $1
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (max_uses = 0 OR uses < max_uses)
            )
            "#,
        )
        .bind(code)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }
}

impl PgInviteRepository {
    /// Find all invites created by a user.
    pub async fn find_by_inviter_id(&self, inviter_id: i64) -> Result<Vec<InviteEntity>, AppError> {
        let invites = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE inviter_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(inviter_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(invites)
    }

    /// Find all invites for a specific channel.
    pub async fn find_by_channel_id(
        &self,
        channel_id: i64,
    ) -> Result<Vec<InviteEntity>, AppError> {
        let invites = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE channel_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(invites)
    }

    /// Find only valid (non-expired, not maxed out) invites for a server.
    pub async fn find_valid_by_server_id(
        &self,
        server_id: i64,
    ) -> Result<Vec<InviteEntity>, AppError> {
        let invites = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE server_id = $1
              AND (expires_at IS NULL OR expires_at > NOW())
              AND (max_uses = 0 OR uses < max_uses)
            ORDER BY created_at DESC
            "#,
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(invites)
    }

    /// Delete all expired invites (cleanup job).
    ///
    /// Returns the number of invites deleted.
    pub async fn delete_expired(&self) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM invites
            WHERE expires_at IS NOT NULL AND expires_at <= NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete all invites for a server.
    ///
    /// Used when deleting a server.
    pub async fn delete_by_server_id(&self, server_id: i64) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM invites WHERE server_id = $1")
            .bind(server_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all invites for a channel.
    ///
    /// Used when deleting a channel.
    pub async fn delete_by_channel_id(&self, channel_id: i64) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM invites WHERE channel_id = $1")
            .bind(channel_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Count invites for a server.
    pub async fn count_by_server_id(&self, server_id: i64) -> Result<i64, AppError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM invites WHERE server_id = $1
            "#,
        )
        .bind(server_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// Get invite metadata including server name.
    ///
    /// Useful for invite preview (before joining).
    pub async fn get_invite_preview(&self, code: &str) -> Result<Option<InvitePreview>, AppError> {
        let preview = sqlx::query_as::<_, InvitePreview>(
            r#"
            SELECT
                i.code,
                s.id as server_id,
                s.name as server_name,
                s.icon_url as server_icon,
                c.id as channel_id,
                c.name as channel_name,
                i.inviter_id,
                u.username as inviter_name,
                i.uses,
                i.max_uses,
                i.expires_at,
                (SELECT COUNT(*) FROM members WHERE server_id = s.id) as member_count
            FROM invites i
            INNER JOIN servers s ON i.server_id = s.id
            INNER JOIN channels c ON i.channel_id = c.id
            LEFT JOIN users u ON i.inviter_id = u.id
            WHERE i.code = $1
              AND (i.expires_at IS NULL OR i.expires_at > NOW())
              AND (i.max_uses = 0 OR i.uses < i.max_uses)
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(preview)
    }

    /// Use an invite (increment uses and return server_id for joining).
    ///
    /// This is an atomic operation that validates and uses the invite.
    /// Returns the server_id if successful.
    pub async fn use_invite(&self, code: &str) -> Result<i64, AppError> {
        // Use a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // Lock and validate the invite
        let invite = sqlx::query_as::<_, InviteEntity>(
            r#"
            SELECT code, server_id, channel_id, inviter_id, max_uses, uses,
                   max_age, temporary, expires_at, created_at
            FROM invites
            WHERE code = $1
            FOR UPDATE
            "#,
        )
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?;

        let invite = invite.ok_or_else(|| AppError::NotFound(format!("Invite {} not found", code)))?;

        // Check if expired
        if let Some(expires_at) = invite.expires_at {
            if expires_at <= Utc::now() {
                return Err(AppError::BadRequest("Invite has expired".to_string()));
            }
        }

        // Check if at max uses
        if invite.max_uses > 0 && invite.uses >= invite.max_uses {
            return Err(AppError::BadRequest(
                "Invite has reached maximum uses".to_string(),
            ));
        }

        // Increment uses
        sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = $1")
            .bind(code)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(invite.server_id)
    }
}

/// Invite preview data for displaying invite information before joining.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InvitePreview {
    pub code: String,
    pub server_id: i64,
    pub server_name: String,
    pub server_icon: Option<String>,
    pub channel_id: i64,
    pub channel_name: String,
    pub inviter_id: Option<i64>,
    pub inviter_name: Option<String>,
    pub uses: i32,
    pub max_uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub member_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_invite_struct() {
        let invite = CreateInvite {
            code: "aBcD1234".to_string(),
            server_id: 123456789,
            channel_id: 987654321,
            inviter_id: Some(111111111),
            max_uses: 10,
            max_age: 86400, // 24 hours
            temporary: false,
        };

        assert_eq!(invite.code, "aBcD1234");
        assert_eq!(invite.max_uses, 10);
        assert_eq!(invite.max_age, 86400);
    }

    #[test]
    fn test_invite_entity_fields() {
        // Just verify the struct can be instantiated
        let _ = InviteEntity {
            code: "test1234".to_string(),
            server_id: 1,
            channel_id: 2,
            inviter_id: Some(3),
            max_uses: 0,
            uses: 0,
            max_age: 0,
            temporary: false,
            expires_at: None,
            created_at: Utc::now(),
        };
    }
}
