//! Role Repository Implementation
//!
//! PostgreSQL implementation of the RoleRepository trait.
//! Handles server roles with permission bitfields.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{Role, RoleRepository};
use crate::shared::error::AppError;

/// Database row representation matching the actual roles table schema.
#[derive(Debug, sqlx::FromRow)]
struct RoleRow {
    id: i64,
    server_id: i64,
    name: String,
    permissions: i64,
    position: i32,
    color: Option<i32>,
    hoist: bool,
    mentionable: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RoleRow {
    /// Convert database row to domain Role entity.
    fn into_role(self) -> Role {
        Role {
            id: self.id,
            server_id: self.server_id,
            name: self.name,
            color: self.color,
            hoist: self.hoist,
            position: self.position,
            permissions: self.permissions,
            mentionable: self.mentionable,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// PostgreSQL role repository implementation.
///
/// Provides CRUD operations for roles against a PostgreSQL database.
#[derive(Clone)]
pub struct PgRoleRepository {
    pool: PgPool,
}

impl PgRoleRepository {
    /// Create a new PgRoleRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all roles assigned to a specific member.
    pub async fn find_by_member(
        &self,
        server_id: i64,
        user_id: i64,
    ) -> Result<Vec<Role>, AppError> {
        let rows = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT r.id, r.server_id, r.name, r.permissions, r.position, r.color,
                   r.hoist, r.mentionable, r.created_at, r.updated_at
            FROM roles r
            INNER JOIN member_roles mr ON r.id = mr.role_id
            WHERE mr.server_id = $1 AND mr.user_id = $2 AND r.deleted_at IS NULL
            ORDER BY r.position DESC
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_role()).collect())
    }

    /// Reorder role positions within a server.
    pub async fn reorder_positions(
        &self,
        server_id: i64,
        positions: Vec<(i64, i32)>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        for (role_id, position) in positions {
            sqlx::query(
                r#"
                UPDATE roles
                SET position = $3, updated_at = NOW()
                WHERE id = $1 AND server_id = $2
                "#,
            )
            .bind(role_id)
            .bind(server_id)
            .bind(position)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepository {
    /// Find a role by its ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        let row = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, server_id, name, permissions, position, color, hoist, mentionable,
                   created_at, updated_at
            FROM roles
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_role()))
    }

    /// Find all roles in a server, ordered by position descending.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<Role>, AppError> {
        let rows = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, server_id, name, permissions, position, color, hoist, mentionable,
                   created_at, updated_at
            FROM roles
            WHERE server_id = $1 AND deleted_at IS NULL
            ORDER BY position DESC
            "#,
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_role()).collect())
    }

    /// Find the @everyone role for a server.
    /// The @everyone role typically has the same ID as the server ID.
    async fn find_everyone_role(&self, server_id: i64) -> Result<Option<Role>, AppError> {
        // Convention: @everyone role has same ID as server, or position = 0
        let row = sqlx::query_as::<_, RoleRow>(
            r#"
            SELECT id, server_id, name, permissions, position, color, hoist, mentionable,
                   created_at, updated_at
            FROM roles
            WHERE server_id = $1 AND (id = $1 OR position = 0)
            ORDER BY position ASC
            LIMIT 1
            "#,
        )
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_role()))
    }

    /// Create a new role.
    async fn create(&self, role: &Role) -> Result<Role, AppError> {
        let row = sqlx::query_as::<_, RoleRow>(
            r#"
            INSERT INTO roles (id, server_id, name, permissions, position, color, hoist, mentionable)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, server_id, name, permissions, position, color, hoist, mentionable,
                      created_at, updated_at
            "#,
        )
        .bind(role.id)
        .bind(role.server_id)
        .bind(&role.name)
        .bind(role.permissions)
        .bind(role.position)
        .bind(role.color)
        .bind(role.hoist)
        .bind(role.mentionable)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("Role with this ID already exists".to_string())
            }
            _ => AppError::Database(e),
        })?;

        Ok(row.into_role())
    }

    /// Update an existing role.
    async fn update(&self, role: &Role) -> Result<Role, AppError> {
        let row = sqlx::query_as::<_, RoleRow>(
            r#"
            UPDATE roles
            SET name = $2,
                permissions = $3,
                position = $4,
                color = $5,
                hoist = $6,
                mentionable = $7,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, server_id, name, permissions, position, color, hoist, mentionable,
                      created_at, updated_at
            "#,
        )
        .bind(role.id)
        .bind(&role.name)
        .bind(role.permissions)
        .bind(role.position)
        .bind(role.color)
        .bind(role.hoist)
        .bind(role.mentionable)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Role with id {} not found", role.id)))?;

        Ok(row.into_role())
    }

    /// Delete a role.
    /// This also removes the role from all members via CASCADE.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Role with id {} not found", id)));
        }

        Ok(())
    }

    /// Update role positions (for reordering).
    async fn update_positions(
        &self,
        server_id: i64,
        positions: Vec<(i64, i32)>,
    ) -> Result<(), AppError> {
        self.reorder_positions(server_id, positions).await
    }

    /// Get the highest role position for a server.
    async fn get_max_position(&self, server_id: i64) -> Result<i32, AppError> {
        let max_pos = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(position) FROM roles WHERE server_id = $1 AND deleted_at IS NULL",
        )
        .bind(server_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(max_pos.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
}
