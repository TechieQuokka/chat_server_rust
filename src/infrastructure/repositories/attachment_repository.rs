//! Attachment Repository Implementation
//!
//! PostgreSQL implementation of file attachment operations.
//! Handles CRUD operations for message attachments with support for various file types.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::shared::error::AppError;

/// Attachment entity representing a file attached to a message.
///
/// Attachments are stored with metadata for efficient retrieval and display.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttachmentEntity {
    /// Snowflake ID for the attachment
    pub id: i64,
    /// ID of the message this attachment belongs to
    pub message_id: i64,
    /// Original filename
    pub filename: String,
    /// MIME content type (e.g., "image/png", "application/pdf")
    pub content_type: Option<String>,
    /// File size in bytes
    pub size: i32,
    /// Primary CDN URL for accessing the file
    pub url: String,
    /// Proxied URL for privacy and caching
    pub proxy_url: Option<String>,
    /// Width in pixels (for images/videos)
    pub width: Option<i32>,
    /// Height in pixels (for images/videos)
    pub height: Option<i32>,
    /// When the attachment was created
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new attachment.
#[derive(Debug, Clone)]
pub struct CreateAttachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i32,
    pub url: String,
    pub proxy_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// Trait defining attachment repository operations.
#[async_trait]
pub trait AttachmentRepository: Send + Sync {
    /// Find an attachment by its ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<AttachmentEntity>, AppError>;

    /// Find all attachments for a message.
    async fn find_by_message_id(&self, message_id: i64) -> Result<Vec<AttachmentEntity>, AppError>;

    /// Create a new attachment.
    async fn create(&self, attachment: &CreateAttachment) -> Result<AttachmentEntity, AppError>;

    /// Delete an attachment by ID.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Delete all attachments for a message.
    async fn delete_by_message_id(&self, message_id: i64) -> Result<u64, AppError>;

    /// Check if an attachment exists.
    async fn exists(&self, id: i64) -> Result<bool, AppError>;
}

/// PostgreSQL implementation of the AttachmentRepository.
pub struct PgAttachmentRepository {
    pool: PgPool,
}

impl PgAttachmentRepository {
    /// Creates a new PgAttachmentRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttachmentRepository for PgAttachmentRepository {
    /// Find an attachment by its ID.
    ///
    /// Returns None if the attachment does not exist.
    async fn find_by_id(&self, id: i64) -> Result<Option<AttachmentEntity>, AppError> {
        let attachment = sqlx::query_as::<_, AttachmentEntity>(
            r#"
            SELECT id, message_id, filename, content_type, size, url,
                   proxy_url, width, height, created_at
            FROM attachments
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(attachment)
    }

    /// Find all attachments for a message.
    ///
    /// Returns attachments ordered by creation time (oldest first).
    async fn find_by_message_id(&self, message_id: i64) -> Result<Vec<AttachmentEntity>, AppError> {
        let attachments = sqlx::query_as::<_, AttachmentEntity>(
            r#"
            SELECT id, message_id, filename, content_type, size, url,
                   proxy_url, width, height, created_at
            FROM attachments
            WHERE message_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(attachments)
    }

    /// Create a new attachment.
    ///
    /// The attachment ID should be a pre-generated Snowflake ID.
    async fn create(&self, attachment: &CreateAttachment) -> Result<AttachmentEntity, AppError> {
        let created = sqlx::query_as::<_, AttachmentEntity>(
            r#"
            INSERT INTO attachments (id, message_id, filename, content_type, size, url, proxy_url, width, height)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, message_id, filename, content_type, size, url,
                      proxy_url, width, height, created_at
            "#,
        )
        .bind(attachment.id)
        .bind(attachment.message_id)
        .bind(&attachment.filename)
        .bind(&attachment.content_type)
        .bind(attachment.size)
        .bind(&attachment.url)
        .bind(&attachment.proxy_url)
        .bind(attachment.width)
        .bind(attachment.height)
        .fetch_one(&self.pool)
        .await?;

        Ok(created)
    }

    /// Delete an attachment by ID.
    ///
    /// Returns an error if the attachment does not exist.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM attachments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Attachment {} not found", id)));
        }

        Ok(())
    }

    /// Delete all attachments for a message.
    ///
    /// Returns the number of attachments deleted.
    async fn delete_by_message_id(&self, message_id: i64) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM attachments WHERE message_id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Check if an attachment exists.
    async fn exists(&self, id: i64) -> Result<bool, AppError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM attachments WHERE id = $1)
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }
}

impl PgAttachmentRepository {
    /// Bulk create attachments for a message.
    ///
    /// More efficient than creating attachments one by one.
    pub async fn bulk_create(
        &self,
        attachments: &[CreateAttachment],
    ) -> Result<Vec<AttachmentEntity>, AppError> {
        if attachments.is_empty() {
            return Ok(Vec::new());
        }

        let mut created = Vec::with_capacity(attachments.len());

        // Use a transaction for atomicity
        let mut tx = self.pool.begin().await?;

        for attachment in attachments {
            let row = sqlx::query_as::<_, AttachmentEntity>(
                r#"
                INSERT INTO attachments (id, message_id, filename, content_type, size, url, proxy_url, width, height)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, message_id, filename, content_type, size, url,
                          proxy_url, width, height, created_at
                "#,
            )
            .bind(attachment.id)
            .bind(attachment.message_id)
            .bind(&attachment.filename)
            .bind(&attachment.content_type)
            .bind(attachment.size)
            .bind(&attachment.url)
            .bind(&attachment.proxy_url)
            .bind(attachment.width)
            .bind(attachment.height)
            .fetch_one(&mut *tx)
            .await?;

            created.push(row);
        }

        tx.commit().await?;

        Ok(created)
    }

    /// Find attachments by content type.
    ///
    /// Useful for filtering by file type (e.g., images only).
    pub async fn find_by_content_type(
        &self,
        channel_id: i64,
        content_type_prefix: &str,
        limit: i32,
    ) -> Result<Vec<AttachmentEntity>, AppError> {
        let limit = limit.min(100).max(1);
        let pattern = format!("{}%", content_type_prefix);

        let attachments = sqlx::query_as::<_, AttachmentEntity>(
            r#"
            SELECT a.id, a.message_id, a.filename, a.content_type, a.size, a.url,
                   a.proxy_url, a.width, a.height, a.created_at
            FROM attachments a
            INNER JOIN messages m ON a.message_id = m.id
            WHERE m.channel_id = $1 AND a.content_type LIKE $2
            ORDER BY a.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(attachments)
    }

    /// Get total attachment size for a message.
    ///
    /// Useful for enforcing upload limits per message.
    pub async fn get_total_size_for_message(&self, message_id: i64) -> Result<i64, AppError> {
        let result: (Option<i64>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(size::bigint), 0)
            FROM attachments
            WHERE message_id = $1
            "#,
        )
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0.unwrap_or(0))
    }

    /// Get attachment count for a message.
    pub async fn count_by_message_id(&self, message_id: i64) -> Result<i64, AppError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM attachments WHERE message_id = $1
            "#,
        )
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// Find images in a channel for gallery view.
    ///
    /// Filters to image/* content types and includes dimensions.
    pub async fn find_images_in_channel(
        &self,
        channel_id: i64,
        before: Option<i64>,
        limit: i32,
    ) -> Result<Vec<AttachmentEntity>, AppError> {
        let limit = limit.min(100).max(1);

        let attachments = match before {
            Some(before_id) => {
                sqlx::query_as::<_, AttachmentEntity>(
                    r#"
                    SELECT a.id, a.message_id, a.filename, a.content_type, a.size, a.url,
                           a.proxy_url, a.width, a.height, a.created_at
                    FROM attachments a
                    INNER JOIN messages m ON a.message_id = m.id
                    WHERE m.channel_id = $1
                      AND a.content_type LIKE 'image/%'
                      AND a.id < $2
                    ORDER BY a.created_at DESC
                    LIMIT $3
                    "#,
                )
                .bind(channel_id)
                .bind(before_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, AttachmentEntity>(
                    r#"
                    SELECT a.id, a.message_id, a.filename, a.content_type, a.size, a.url,
                           a.proxy_url, a.width, a.height, a.created_at
                    FROM attachments a
                    INNER JOIN messages m ON a.message_id = m.id
                    WHERE m.channel_id = $1
                      AND a.content_type LIKE 'image/%'
                    ORDER BY a.created_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(channel_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(attachments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_attachment_struct() {
        let attachment = CreateAttachment {
            id: 123456789,
            message_id: 987654321,
            filename: "test.png".to_string(),
            content_type: Some("image/png".to_string()),
            size: 1024,
            url: "https://cdn.example.com/test.png".to_string(),
            proxy_url: Some("https://proxy.example.com/test.png".to_string()),
            width: Some(800),
            height: Some(600),
        };

        assert_eq!(attachment.filename, "test.png");
        assert_eq!(attachment.size, 1024);
        assert_eq!(attachment.width, Some(800));
    }
}
