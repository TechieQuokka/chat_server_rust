//! Channel Repository Implementation
//!
//! PostgreSQL implementation of the ChannelRepository trait.
//! Handles text channels, voice channels, categories, and DMs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{Channel, ChannelRepository, ChannelType, PermissionOverwrite};
use crate::shared::error::AppError;

/// Database row representation matching the actual channels table schema.
#[derive(Debug, sqlx::FromRow)]
struct ChannelRow {
    id: i64,
    server_id: Option<i64>,
    name: String,
    #[sqlx(rename = "type")]
    channel_type: String, // PostgreSQL ENUM comes as string
    topic: Option<String>,
    position: i32,
    parent_id: Option<i64>,
    nsfw: bool,
    rate_limit_per_user: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ChannelRow {
    /// Convert database row to domain Channel entity.
    fn into_channel(self) -> Channel {
        Channel {
            id: self.id,
            server_id: self.server_id,
            name: self.name,
            channel_type: ChannelType::from_str(&self.channel_type),
            topic: self.topic,
            position: self.position,
            parent_id: self.parent_id,
            nsfw: self.nsfw,
            rate_limit_per_user: self.rate_limit_per_user.unwrap_or(0),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Database row for permission overwrites.
#[derive(Debug, sqlx::FromRow)]
struct PermissionOverwriteRow {
    channel_id: i64,
    target_type: String,
    target_id: i64,
    allow: i64,
    deny: i64,
}

impl PermissionOverwriteRow {
    fn into_permission_overwrite(self) -> PermissionOverwrite {
        PermissionOverwrite {
            channel_id: self.channel_id,
            target_id: self.target_id,
            target_type: self.target_type,
            allow: self.allow,
            deny: self.deny,
        }
    }
}

/// PostgreSQL channel repository implementation.
///
/// Provides CRUD operations for channels against a PostgreSQL database.
#[derive(Clone)]
pub struct PgChannelRepository {
    pool: PgPool,
}

impl PgChannelRepository {
    /// Create a new PgChannelRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Reorder channel positions within a server.
    pub async fn reorder_positions(
        &self,
        server_id: i64,
        positions: Vec<(i64, i32)>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        for (channel_id, position) in positions {
            sqlx::query(
                r#"
                UPDATE channels
                SET position = $3, updated_at = NOW()
                WHERE id = $1 AND server_id = $2
                "#,
            )
            .bind(channel_id)
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
impl ChannelRepository for PgChannelRepository {
    /// Find a channel by its ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Channel>, AppError> {
        let row = sqlx::query_as::<_, ChannelRow>(
            r#"
            SELECT id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user,
                   created_at, updated_at
            FROM channels
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_channel()))
    }

    /// Find all channels in a server, ordered by position.
    async fn find_by_server_id(&self, server_id: i64) -> Result<Vec<Channel>, AppError> {
        let rows = sqlx::query_as::<_, ChannelRow>(
            r#"
            SELECT id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user,
                   created_at, updated_at
            FROM channels
            WHERE server_id = $1 AND deleted_at IS NULL
            ORDER BY position ASC
            "#,
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_channel()).collect())
    }

    /// Find channels by parent category ID.
    async fn find_by_parent_id(&self, parent_id: i64) -> Result<Vec<Channel>, AppError> {
        let rows = sqlx::query_as::<_, ChannelRow>(
            r#"
            SELECT id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user,
                   created_at, updated_at
            FROM channels
            WHERE parent_id = $1 AND deleted_at IS NULL
            ORDER BY position ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_channel()).collect())
    }

    /// Find DM channel between two users.
    /// Note: Current schema doesn't fully support DM channels with recipients table.
    async fn find_dm_channel(&self, _user1_id: i64, _user2_id: i64) -> Result<Option<Channel>, AppError> {
        // DM channel lookup would require a separate dm_participants table
        // For now, return None as the schema doesn't fully support this
        Ok(None)
    }

    /// Create a new channel.
    async fn create(&self, channel: &Channel) -> Result<Channel, AppError> {
        let channel_type_str = channel.channel_type.as_str();

        let row = sqlx::query_as::<_, ChannelRow>(
            r#"
            INSERT INTO channels (id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user)
            VALUES ($1, $2, $3, $4::channel_type, $5, $6, $7, $8, $9)
            RETURNING id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user,
                      created_at, updated_at
            "#,
        )
        .bind(channel.id)
        .bind(channel.server_id)
        .bind(&channel.name)
        .bind(channel_type_str)
        .bind(&channel.topic)
        .bind(channel.position)
        .bind(channel.parent_id)
        .bind(channel.nsfw)
        .bind(channel.rate_limit_per_user)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("Channel with this ID already exists".to_string())
            }
            _ => AppError::Database(e),
        })?;

        Ok(row.into_channel())
    }

    /// Update an existing channel.
    async fn update(&self, channel: &Channel) -> Result<Channel, AppError> {
        let row = sqlx::query_as::<_, ChannelRow>(
            r#"
            UPDATE channels
            SET name = $2,
                topic = $3,
                position = $4,
                parent_id = $5,
                nsfw = $6,
                rate_limit_per_user = $7,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, server_id, name, type, topic, position, parent_id, nsfw, rate_limit_per_user,
                      created_at, updated_at
            "#,
        )
        .bind(channel.id)
        .bind(&channel.name)
        .bind(&channel.topic)
        .bind(channel.position)
        .bind(channel.parent_id)
        .bind(channel.nsfw)
        .bind(channel.rate_limit_per_user)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Channel with id {} not found", channel.id)))?;

        Ok(row.into_channel())
    }

    /// Delete a channel.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM channels WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Channel with id {} not found", id)));
        }

        Ok(())
    }

    /// Update channel positions (for reordering).
    async fn update_positions(
        &self,
        server_id: i64,
        positions: Vec<(i64, i32)>,
    ) -> Result<(), AppError> {
        self.reorder_positions(server_id, positions).await
    }

    /// Get permission overwrites for a channel.
    async fn get_permission_overwrites(&self, channel_id: i64) -> Result<Vec<PermissionOverwrite>, AppError> {
        let rows = sqlx::query_as::<_, PermissionOverwriteRow>(
            r#"
            SELECT channel_id, target_type, target_id, allow, deny
            FROM channel_permission_overwrites
            WHERE channel_id = $1
            "#,
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_permission_overwrite()).collect())
    }

    /// Set permission overwrites for a channel.
    /// Replaces all existing overwrites.
    async fn set_permission_overwrites(
        &self,
        channel_id: i64,
        overwrites: Vec<PermissionOverwrite>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // Delete existing overwrites
        sqlx::query("DELETE FROM channel_permission_overwrites WHERE channel_id = $1")
            .bind(channel_id)
            .execute(&mut *tx)
            .await?;

        // Insert new overwrites
        for overwrite in overwrites {
            sqlx::query(
                r#"
                INSERT INTO channel_permission_overwrites (channel_id, target_type, target_id, allow, deny)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(channel_id)
            .bind(&overwrite.target_type)
            .bind(overwrite.target_id)
            .bind(overwrite.allow)
            .bind(overwrite.deny)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
}
