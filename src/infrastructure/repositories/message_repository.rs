//! Message Repository Implementation
//!
//! PostgreSQL implementation of message operations with cursor-based pagination,
//! bulk operations, and efficient querying for chat applications.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{Attachment, Message, MessageRepository, MessageType};
use crate::shared::error::AppError;

/// PostgreSQL message repository implementation.
///
/// Provides efficient message storage and retrieval with:
/// - Cursor-based pagination for infinite scroll
/// - Bulk delete operations
/// - Pin/unpin functionality
pub struct PgMessageRepository {
    pool: PgPool,
}

impl PgMessageRepository {
    /// Creates a new PgMessageRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal row type for message queries.
/// Maps to the messages table schema defined in the migration.
#[derive(Debug, sqlx::FromRow)]
struct MessageRow {
    id: i64,
    channel_id: i64,
    author_id: i64,
    content: String,
    message_type: String, // PostgreSQL enum maps to string
    reply_to_id: Option<i64>,
    pinned: bool,
    edited_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl MessageRow {
    /// Converts database row to domain Message entity.
    fn into_message(self) -> Message {
        Message {
            id: self.id,
            channel_id: self.channel_id,
            author_id: self.author_id,
            content: self.content,
            message_type: MessageType::from_str(&self.message_type),
            reply_to_id: self.reply_to_id,
            pinned: self.pinned,
            edited_at: self.edited_at,
            created_at: self.created_at,
        }
    }
}

/// Internal row type for attachment queries.
#[derive(Debug, sqlx::FromRow)]
struct AttachmentRow {
    id: i64,
    message_id: i64,
    filename: String,
    content_type: Option<String>,
    size: i32,
    url: String,
    proxy_url: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    created_at: DateTime<Utc>,
}

impl AttachmentRow {
    fn into_attachment(self) -> Attachment {
        Attachment {
            id: self.id,
            message_id: self.message_id,
            filename: self.filename,
            content_type: self.content_type,
            size: self.size,
            url: self.url,
            proxy_url: self.proxy_url,
            height: self.height,
            width: self.width,
            created_at: self.created_at,
        }
    }
}

#[async_trait]
impl MessageRepository for PgMessageRepository {
    /// Find a message by its ID.
    ///
    /// Returns None if the message does not exist.
    async fn find_by_id(&self, id: i64) -> Result<Option<Message>, AppError> {
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, channel_id, author_id, content,
                   message_type::text as message_type, reply_to_id,
                   pinned, edited_at, created_at
            FROM messages
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_message()))
    }

    /// Find messages in a channel with cursor-based pagination.
    ///
    /// Uses keyset pagination for efficient scrolling through large message histories.
    /// Messages are returned in descending order (newest first).
    ///
    /// # Arguments
    /// * `channel_id` - The channel to fetch messages from
    /// * `before` - Cursor: fetch messages older than this message ID
    /// * `after` - Cursor: fetch messages newer than this message ID
    /// * `limit` - Maximum number of messages to return (capped at 100)
    async fn find_by_channel(
        &self,
        channel_id: i64,
        before: Option<i64>,
        after: Option<i64>,
        limit: i32,
    ) -> Result<Vec<Message>, AppError> {
        // Cap limit to prevent excessive queries
        let limit = limit.min(100).max(1);

        let rows = match (before, after) {
            (Some(before_id), None) => {
                // Cursor-based pagination: get messages before cursor
                sqlx::query_as::<_, MessageRow>(
                    r#"
                    SELECT id, channel_id, author_id, content,
                           message_type::text as message_type, reply_to_id,
                           pinned, edited_at, created_at
                    FROM messages
                    WHERE channel_id = $1 AND id < $2
                    ORDER BY id DESC
                    LIMIT $3
                    "#,
                )
                .bind(channel_id)
                .bind(before_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(after_id)) => {
                // Get messages after cursor (newer messages)
                sqlx::query_as::<_, MessageRow>(
                    r#"
                    SELECT id, channel_id, author_id, content,
                           message_type::text as message_type, reply_to_id,
                           pinned, edited_at, created_at
                    FROM messages
                    WHERE channel_id = $1 AND id > $2
                    ORDER BY id ASC
                    LIMIT $3
                    "#,
                )
                .bind(channel_id)
                .bind(after_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            _ => {
                // No cursor: get most recent messages
                sqlx::query_as::<_, MessageRow>(
                    r#"
                    SELECT id, channel_id, author_id, content,
                           message_type::text as message_type, reply_to_id,
                           pinned, edited_at, created_at
                    FROM messages
                    WHERE channel_id = $1
                    ORDER BY id DESC
                    LIMIT $2
                    "#,
                )
                .bind(channel_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let messages: Vec<Message> = rows.into_iter().map(|r| r.into_message()).collect();
        Ok(messages)
    }

    /// Find all pinned messages in a channel.
    ///
    /// Returns messages ordered by when they were created.
    async fn find_pinned(&self, channel_id: i64) -> Result<Vec<Message>, AppError> {
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, channel_id, author_id, content,
                   message_type::text as message_type, reply_to_id,
                   pinned, edited_at, created_at
            FROM messages
            WHERE channel_id = $1 AND pinned = TRUE
            ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        let messages: Vec<Message> = rows.into_iter().map(|r| r.into_message()).collect();
        Ok(messages)
    }

    /// Create a new message.
    ///
    /// The message ID should be a pre-generated Snowflake ID from the application layer.
    async fn create(&self, message: &Message) -> Result<Message, AppError> {
        let message_type_str = message.message_type.as_str();

        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            INSERT INTO messages (id, channel_id, author_id, content, message_type, reply_to_id, pinned)
            VALUES ($1, $2, $3, $4, $5::message_type, $6, $7)
            RETURNING id, channel_id, author_id, content,
                      message_type::text as message_type, reply_to_id,
                      pinned, edited_at, created_at
            "#,
        )
        .bind(message.id)
        .bind(message.channel_id)
        .bind(message.author_id)
        .bind(&message.content)
        .bind(message_type_str)
        .bind(message.reply_to_id)
        .bind(message.pinned)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into_message())
    }

    /// Update a message (for editing content).
    ///
    /// Only content can be edited. The edited_at timestamp is automatically updated.
    async fn update(&self, message: &Message) -> Result<Message, AppError> {
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            UPDATE messages
            SET content = $2, edited_at = NOW()
            WHERE id = $1
            RETURNING id, channel_id, author_id, content,
                      message_type::text as message_type, reply_to_id,
                      pinned, edited_at, created_at
            "#,
        )
        .bind(message.id)
        .bind(&message.content)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into_message())
    }

    /// Delete a message.
    ///
    /// This permanently removes the message. Consider soft deletes for production.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Message {} not found", id)));
        }

        Ok(())
    }

    /// Bulk delete multiple messages in a channel.
    ///
    /// This is more efficient than deleting messages one by one.
    /// Only deletes messages that belong to the specified channel.
    async fn bulk_delete(&self, channel_id: i64, message_ids: Vec<i64>) -> Result<(), AppError> {
        if message_ids.is_empty() {
            return Ok(());
        }

        sqlx::query(
            r#"
            DELETE FROM messages
            WHERE channel_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(channel_id)
        .bind(&message_ids)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Pin a message.
    async fn pin(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE messages SET pinned = TRUE WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Message {} not found", id)));
        }

        Ok(())
    }

    /// Unpin a message.
    async fn unpin(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE messages SET pinned = FALSE WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Message {} not found", id)));
        }

        Ok(())
    }

    /// Find messages by author in a channel.
    ///
    /// Returns messages from a specific author, ordered by newest first.
    async fn find_by_author(
        &self,
        channel_id: i64,
        author_id: i64,
        limit: i32,
    ) -> Result<Vec<Message>, AppError> {
        let limit = limit.min(100).max(1);

        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, channel_id, author_id, content,
                   message_type::text as message_type, reply_to_id,
                   pinned, edited_at, created_at
            FROM messages
            WHERE channel_id = $1 AND author_id = $2
            ORDER BY id DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(author_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let messages: Vec<Message> = rows.into_iter().map(|r| r.into_message()).collect();
        Ok(messages)
    }

    /// Get the count of messages in a channel.
    async fn count_by_channel(&self, channel_id: i64) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM messages WHERE channel_id = $1",
        )
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

impl PgMessageRepository {
    /// Load attachments for a message.
    #[allow(dead_code)]
    async fn load_attachments(&self, message_id: i64) -> Result<Vec<Attachment>, AppError> {
        let rows = sqlx::query_as::<_, AttachmentRow>(
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

        Ok(rows.into_iter().map(|r| r.into_attachment()).collect())
    }

    /// Find messages by channel with cursor pagination.
    /// This is a convenience method matching the requested API signature.
    pub async fn find_by_channel_id(
        &self,
        channel_id: i64,
        limit: i32,
        before: Option<i64>,
    ) -> Result<Vec<Message>, AppError> {
        self.find_by_channel(channel_id, before, None, limit).await
    }

    /// Get pinned messages for a channel.
    /// This is a convenience method matching the requested API signature.
    pub async fn get_pinned_messages(&self, channel_id: i64) -> Result<Vec<Message>, AppError> {
        self.find_pinned(channel_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert!(matches!(MessageType::from_str("default"), MessageType::Default));
        assert!(matches!(MessageType::from_str("reply"), MessageType::Reply));
        assert!(matches!(MessageType::from_str("guild_member_join"), MessageType::GuildMemberJoin));
        assert!(matches!(MessageType::from_str("unknown"), MessageType::Default));
    }

    #[test]
    fn test_message_type_to_str() {
        assert_eq!(MessageType::Default.as_str(), "default");
        assert_eq!(MessageType::Reply.as_str(), "reply");
        assert_eq!(MessageType::GuildMemberJoin.as_str(), "guild_member_join");
    }
}
