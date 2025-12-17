//! Server (Guild) Repository Implementation
//!
//! PostgreSQL implementation of the GuildRepository trait.
//! Note: The database uses "servers" table while the domain calls it "Guild" (Discord terminology).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{Server, ServerRepository};
use crate::shared::error::AppError;

/// Database row representation matching the actual servers table schema.
#[derive(Debug, sqlx::FromRow)]
struct ServerRow {
    id: i64,
    name: String,
    owner_id: i64,
    icon_url: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ServerRow {
    /// Convert database row to domain Server entity.
    /// Populates additional domain fields with sensible defaults.
    fn into_server(self) -> Server {
        Server {
            id: self.id,
            name: self.name,
            owner_id: self.owner_id,
            icon_url: self.icon_url,
            description: self.description,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// PostgreSQL server (guild) repository implementation.
///
/// Provides CRUD operations for servers against a PostgreSQL database.
/// The database table is named "servers" while the domain entity is "Guild".
#[derive(Clone)]
pub struct PgServerRepository {
    pool: PgPool,
}

impl PgServerRepository {
    /// Create a new PgServerRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl ServerRepository for PgServerRepository {
    /// Find a server by its ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Server>, AppError> {
        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, name, owner_id, icon_url, description, created_at, updated_at
            FROM servers
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_server()))
    }

    /// Find all servers a user is a member of.
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Server>, AppError> {
        let rows = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT s.id, s.name, s.owner_id, s.icon_url, s.description, s.created_at, s.updated_at
            FROM servers s
            INNER JOIN server_members sm ON s.id = sm.server_id
            WHERE sm.user_id = $1
            ORDER BY sm.joined_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_server()).collect())
    }

    /// Find all servers owned by a user.
    async fn find_by_owner_id(&self, owner_id: i64) -> Result<Vec<Server>, AppError> {
        let rows = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, name, owner_id, icon_url, description, created_at, updated_at
            FROM servers
            WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_server()).collect())
    }

    /// Create a new server.
    async fn create(&self, server: &Server) -> Result<Server, AppError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            INSERT INTO servers (id, name, owner_id, icon_url, description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, owner_id, icon_url, description, created_at, updated_at
            "#,
        )
        .bind(server.id)
        .bind(&server.name)
        .bind(server.owner_id)
        .bind(&server.icon_url)
        .bind(&server.description)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("Server with this ID already exists".to_string())
            }
            _ => AppError::Database(e),
        })?;

        // Add owner as the first member
        sqlx::query(
            r#"
            INSERT INTO server_members (server_id, user_id, nickname, joined_at)
            VALUES ($1, $2, NULL, NOW())
            ON CONFLICT (server_id, user_id) DO NOTHING
            "#,
        )
        .bind(server.id)
        .bind(server.owner_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(row.into_server())
    }

    /// Update an existing server.
    async fn update(&self, server: &Server) -> Result<Server, AppError> {
        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            UPDATE servers
            SET name = $2,
                icon_url = $3,
                description = $4,
                owner_id = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, owner_id, icon_url, description, created_at, updated_at
            "#,
        )
        .bind(server.id)
        .bind(&server.name)
        .bind(&server.icon_url)
        .bind(&server.description)
        .bind(server.owner_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Server with id {} not found", server.id)))?;

        Ok(row.into_server())
    }

    /// Delete a server.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM servers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Server with id {} not found", id)));
        }

        Ok(())
    }

    /// Get the member count for a server.
    async fn get_member_count(&self, id: i64) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM server_members WHERE server_id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Transfer ownership to another user.
    async fn transfer_ownership(&self, server_id: i64, new_owner_id: i64) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE servers
            SET owner_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(server_id)
        .bind(new_owner_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Server with id {} not found", server_id)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
}
