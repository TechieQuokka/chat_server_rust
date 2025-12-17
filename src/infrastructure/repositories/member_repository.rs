//! Member Repository Implementation
//!
//! PostgreSQL implementation of the MemberRepository trait.
//! Handles server membership and role assignments.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{Member, MemberRepository};
use crate::shared::error::AppError;

/// Database row representation matching the actual server_members table schema.
#[derive(Debug, sqlx::FromRow)]
struct MemberRow {
    server_id: i64,
    user_id: i64,
    nickname: Option<String>,
    joined_at: DateTime<Utc>,
}

impl MemberRow {
    /// Convert database row to domain Member entity.
    /// Note: roles are loaded separately.
    fn into_member(self, roles: Vec<i64>) -> Member {
        Member {
            server_id: self.server_id,
            user_id: self.user_id,
            nickname: self.nickname,
            joined_at: self.joined_at,
            roles,
        }
    }
}

/// PostgreSQL member repository implementation.
///
/// Provides CRUD operations for server members against a PostgreSQL database.
#[derive(Clone)]
pub struct PgMemberRepository {
    pool: PgPool,
}

impl PgMemberRepository {
    /// Create a new PgMemberRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Helper to load roles for a member.
    async fn load_member_roles(&self, server_id: i64, user_id: i64) -> Result<Vec<i64>, AppError> {
        let roles = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT role_id FROM member_roles
            WHERE server_id = $1 AND user_id = $2
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    /// Update a member's nickname.
    pub async fn update_nickname(
        &self,
        server_id: i64,
        user_id: i64,
        nickname: Option<&str>,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE server_members
            SET nickname = $3
            WHERE server_id = $1 AND user_id = $2
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .bind(nickname)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Member not found in server {} for user {}",
                server_id, user_id
            )));
        }

        Ok(())
    }

    /// Get all roles for a member (alias for get_roles in trait).
    pub async fn get_member_roles(
        &self,
        server_id: i64,
        user_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.get_roles(server_id, user_id).await
    }
}

#[async_trait]
impl MemberRepository for PgMemberRepository {
    /// Find a member by server and user ID.
    async fn find(&self, server_id: i64, user_id: i64) -> Result<Option<Member>, AppError> {
        let row = sqlx::query_as::<_, MemberRow>(
            r#"
            SELECT server_id, user_id, nickname, joined_at
            FROM server_members
            WHERE server_id = $1 AND user_id = $2
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let roles = self.load_member_roles(server_id, user_id).await?;
                Ok(Some(r.into_member(roles)))
            }
            None => Ok(None),
        }
    }

    /// Find all servers a user is a member of.
    async fn find_by_user(&self, user_id: i64) -> Result<Vec<Member>, AppError> {
        let rows = sqlx::query_as::<_, MemberRow>(
            r#"
            SELECT server_id, user_id, nickname, joined_at
            FROM server_members
            WHERE user_id = $1
            ORDER BY joined_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            let roles = self.load_member_roles(row.server_id, user_id).await?;
            members.push(row.into_member(roles));
        }

        Ok(members)
    }

    /// Find all members in a server with cursor-based pagination.
    async fn find_by_server_id(
        &self,
        server_id: i64,
        after: Option<i64>,
        limit: i32,
    ) -> Result<Vec<Member>, AppError> {
        let rows = if let Some(after_user_id) = after {
            // Cursor-based pagination using user_id
            sqlx::query_as::<_, MemberRow>(
                r#"
                SELECT server_id, user_id, nickname, joined_at
                FROM server_members
                WHERE server_id = $1 AND user_id > $2
                ORDER BY user_id ASC
                LIMIT $3
                "#,
            )
            .bind(server_id)
            .bind(after_user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, MemberRow>(
                r#"
                SELECT server_id, user_id, nickname, joined_at
                FROM server_members
                WHERE server_id = $1
                ORDER BY user_id ASC
                LIMIT $2
                "#,
            )
            .bind(server_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            let roles = self.load_member_roles(server_id, row.user_id).await?;
            members.push(row.into_member(roles));
        }

        Ok(members)
    }

    /// Search members by nickname or username.
    /// Joins with users table to search by username as well.
    async fn search(
        &self,
        server_id: i64,
        query: &str,
        limit: i32,
    ) -> Result<Vec<Member>, AppError> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query_as::<_, MemberRow>(
            r#"
            SELECT sm.server_id, sm.user_id, sm.nickname, sm.joined_at
            FROM server_members sm
            INNER JOIN users u ON sm.user_id = u.id
            WHERE sm.server_id = $1
              AND (sm.nickname ILIKE $2 OR u.username ILIKE $2)
            ORDER BY sm.joined_at DESC
            LIMIT $3
            "#,
        )
        .bind(server_id)
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            let roles = self.load_member_roles(server_id, row.user_id).await?;
            members.push(row.into_member(roles));
        }

        Ok(members)
    }

    /// Add a member to a server.
    async fn create(&self, member: &Member) -> Result<Member, AppError> {
        let mut tx = self.pool.begin().await?;

        // Insert the member
        let row = sqlx::query_as::<_, MemberRow>(
            r#"
            INSERT INTO server_members (server_id, user_id, nickname, joined_at)
            VALUES ($1, $2, $3, $4)
            RETURNING server_id, user_id, nickname, joined_at
            "#,
        )
        .bind(member.server_id)
        .bind(member.user_id)
        .bind(&member.nickname)
        .bind(member.joined_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("User is already a member of this server".to_string())
            }
            _ => AppError::Database(e),
        })?;

        // Add any initial roles
        for role_id in &member.roles {
            sqlx::query(
                r#"
                INSERT INTO member_roles (server_id, user_id, role_id)
                VALUES ($1, $2, $3)
                ON CONFLICT (server_id, user_id, role_id) DO NOTHING
                "#,
            )
            .bind(member.server_id)
            .bind(member.user_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(row.into_member(member.roles.clone()))
    }

    /// Update a member.
    async fn update(&self, member: &Member) -> Result<Member, AppError> {
        // Update nickname only (roles are managed separately)
        let result = sqlx::query(
            r#"
            UPDATE server_members
            SET nickname = $3
            WHERE server_id = $1 AND user_id = $2
            "#,
        )
        .bind(member.server_id)
        .bind(member.user_id)
        .bind(&member.nickname)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Member not found in server {} for user {}",
                member.server_id, member.user_id
            )));
        }

        // Return updated member with current roles
        let roles = self.load_member_roles(member.server_id, member.user_id).await?;
        let mut updated = member.clone();
        updated.roles = roles;
        Ok(updated)
    }

    /// Remove a member from a server.
    async fn delete(&self, server_id: i64, user_id: i64) -> Result<(), AppError> {
        // member_roles will be deleted via CASCADE
        let result = sqlx::query(
            "DELETE FROM server_members WHERE server_id = $1 AND user_id = $2",
        )
        .bind(server_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Member not found in server {} for user {}",
                server_id, user_id
            )));
        }

        Ok(())
    }

    /// Add a role to a member.
    async fn add_role(&self, server_id: i64, user_id: i64, role_id: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO member_roles (server_id, user_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (server_id, user_id, role_id) DO NOTHING
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .bind(role_id)
        .execute(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
                AppError::NotFound("Member or role not found".to_string())
            }
            _ => AppError::Database(e),
        })?;

        Ok(())
    }

    /// Remove a role from a member.
    async fn remove_role(&self, server_id: i64, user_id: i64, role_id: i64) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM member_roles WHERE server_id = $1 AND user_id = $2 AND role_id = $3",
        )
        .bind(server_id)
        .bind(user_id)
        .bind(role_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all role IDs for a member.
    async fn get_roles(&self, server_id: i64, user_id: i64) -> Result<Vec<i64>, AppError> {
        self.load_member_roles(server_id, user_id).await
    }

    /// Check if a user is a member of a server.
    async fn is_member(&self, server_id: i64, user_id: i64) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2)",
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get the member count for a server.
    async fn count_by_server(&self, server_id: i64) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM server_members WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Find members with a specific role.
    async fn find_by_role(&self, server_id: i64, role_id: i64) -> Result<Vec<Member>, AppError> {
        let rows = sqlx::query_as::<_, MemberRow>(
            r#"
            SELECT sm.server_id, sm.user_id, sm.nickname, sm.joined_at
            FROM server_members sm
            INNER JOIN member_roles mr ON sm.server_id = mr.server_id AND sm.user_id = mr.user_id
            WHERE sm.server_id = $1 AND mr.role_id = $2
            ORDER BY sm.joined_at DESC
            "#,
        )
        .bind(server_id)
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            let roles = self.load_member_roles(server_id, row.user_id).await?;
            members.push(row.into_member(roles));
        }

        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
}
