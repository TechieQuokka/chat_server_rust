//! Reaction Repository Implementation
//!
//! PostgreSQL implementation of message reaction operations.
//! Reactions are stored per-user per-emoji per-message with efficient aggregation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::shared::error::AppError;

/// Aggregated reaction data for display.
///
/// Represents a group of reactions with the same emoji on a message.
#[derive(Debug, Clone)]
pub struct ReactionGroup {
    /// The emoji identifier (Unicode or custom emoji ID)
    pub emoji: String,
    /// Total count of users who reacted with this emoji
    pub count: i64,
    /// When the first reaction with this emoji was added
    pub first_reaction_at: DateTime<Utc>,
}

/// Individual reaction record from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageReaction {
    pub message_id: i64,
    pub user_id: i64,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

/// Trait defining reaction repository operations.
#[async_trait]
pub trait ReactionRepository: Send + Sync {
    /// Add a reaction to a message.
    ///
    /// Idempotent: adding the same reaction twice has no effect.
    async fn add_reaction(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<(), AppError>;

    /// Remove a reaction from a message.
    async fn remove_reaction(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<(), AppError>;

    /// Get all reactions on a message, grouped by emoji.
    ///
    /// Returns aggregated counts per emoji type.
    async fn get_reactions(&self, message_id: i64) -> Result<Vec<ReactionGroup>, AppError>;

    /// Get all user IDs who reacted with a specific emoji.
    ///
    /// Returns user IDs in chronological order (oldest first).
    async fn get_users_for_reaction(
        &self,
        message_id: i64,
        emoji: &str,
    ) -> Result<Vec<i64>, AppError>;

    /// Check if a user has reacted with a specific emoji.
    async fn has_user_reacted(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<bool, AppError>;

    /// Remove all reactions from a message.
    ///
    /// Used when deleting a message or clearing all reactions.
    async fn remove_all_reactions(&self, message_id: i64) -> Result<(), AppError>;

    /// Remove all reactions with a specific emoji from a message.
    async fn remove_all_reactions_for_emoji(
        &self,
        message_id: i64,
        emoji: &str,
    ) -> Result<(), AppError>;

    /// Get all reactions by a user across messages in a channel.
    ///
    /// Useful for "your reactions" features.
    async fn get_user_reactions_in_channel(
        &self,
        channel_id: i64,
        user_id: i64,
        limit: i32,
    ) -> Result<Vec<MessageReaction>, AppError>;
}

/// PostgreSQL implementation of the ReactionRepository.
pub struct PgReactionRepository {
    pool: PgPool,
}

impl PgReactionRepository {
    /// Creates a new PgReactionRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal row type for reaction group queries.
#[derive(Debug, sqlx::FromRow)]
struct ReactionGroupRow {
    emoji: String,
    count: i64,
    first_reaction_at: DateTime<Utc>,
}

#[async_trait]
impl ReactionRepository for PgReactionRepository {
    /// Add a reaction to a message.
    ///
    /// Uses INSERT ON CONFLICT to make the operation idempotent.
    /// If the user already reacted with this emoji, no change occurs.
    async fn add_reaction(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO message_reactions (message_id, user_id, emoji)
            VALUES ($1, $2, $3)
            ON CONFLICT (message_id, user_id, emoji) DO NOTHING
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a reaction from a message.
    ///
    /// Silently succeeds if the reaction doesn't exist.
    async fn remove_reaction(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM message_reactions
            WHERE message_id = $1 AND user_id = $2 AND emoji = $3
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all reactions on a message, grouped by emoji.
    ///
    /// Returns reactions ordered by when each emoji was first used.
    async fn get_reactions(&self, message_id: i64) -> Result<Vec<ReactionGroup>, AppError> {
        let rows = sqlx::query_as::<_, ReactionGroupRow>(
            r#"
            SELECT
                emoji,
                COUNT(*) as count,
                MIN(created_at) as first_reaction_at
            FROM message_reactions
            WHERE message_id = $1
            GROUP BY emoji
            ORDER BY first_reaction_at ASC
            "#,
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ReactionGroup {
                emoji: r.emoji,
                count: r.count,
                first_reaction_at: r.first_reaction_at,
            })
            .collect())
    }

    /// Get all user IDs who reacted with a specific emoji.
    ///
    /// Returns users in the order they reacted (oldest first).
    /// Useful for displaying "X, Y, Z and N others reacted".
    async fn get_users_for_reaction(
        &self,
        message_id: i64,
        emoji: &str,
    ) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            r#"
            SELECT user_id
            FROM message_reactions
            WHERE message_id = $1 AND emoji = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(message_id)
        .bind(emoji)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Check if a user has reacted with a specific emoji.
    ///
    /// Efficient single-row check using EXISTS.
    async fn has_user_reacted(
        &self,
        message_id: i64,
        user_id: i64,
        emoji: &str,
    ) -> Result<bool, AppError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM message_reactions
                WHERE message_id = $1 AND user_id = $2 AND emoji = $3
            )
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// Remove all reactions from a message.
    async fn remove_all_reactions(&self, message_id: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM message_reactions
            WHERE message_id = $1
            "#,
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove all reactions with a specific emoji from a message.
    ///
    /// Used by moderators to remove specific reaction types.
    async fn remove_all_reactions_for_emoji(
        &self,
        message_id: i64,
        emoji: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM message_reactions
            WHERE message_id = $1 AND emoji = $2
            "#,
        )
        .bind(message_id)
        .bind(emoji)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all reactions by a user in a channel.
    ///
    /// Requires joining with messages table to filter by channel.
    async fn get_user_reactions_in_channel(
        &self,
        channel_id: i64,
        user_id: i64,
        limit: i32,
    ) -> Result<Vec<MessageReaction>, AppError> {
        let limit = limit.min(100).max(1);

        let rows = sqlx::query_as::<_, MessageReaction>(
            r#"
            SELECT mr.message_id, mr.user_id, mr.emoji, mr.created_at
            FROM message_reactions mr
            INNER JOIN messages m ON mr.message_id = m.id
            WHERE m.channel_id = $1 AND mr.user_id = $2
            ORDER BY mr.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

impl PgReactionRepository {
    /// Get reaction counts with user's reaction status.
    ///
    /// Returns reaction groups with a flag indicating if the specified user
    /// has reacted with each emoji. Useful for rendering the "me" field.
    pub async fn get_reactions_with_user_status(
        &self,
        message_id: i64,
        user_id: i64,
    ) -> Result<Vec<(ReactionGroup, bool)>, AppError> {
        #[derive(sqlx::FromRow)]
        struct ReactionWithStatus {
            emoji: String,
            count: i64,
            first_reaction_at: DateTime<Utc>,
            user_reacted: bool,
        }

        let rows = sqlx::query_as::<_, ReactionWithStatus>(
            r#"
            SELECT
                emoji,
                COUNT(*) as count,
                MIN(created_at) as first_reaction_at,
                BOOL_OR(user_id = $2) as user_reacted
            FROM message_reactions
            WHERE message_id = $1
            GROUP BY emoji
            ORDER BY first_reaction_at ASC
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    ReactionGroup {
                        emoji: r.emoji,
                        count: r.count,
                        first_reaction_at: r.first_reaction_at,
                    },
                    r.user_reacted,
                )
            })
            .collect())
    }

    /// Bulk add reactions (for message import/migration).
    ///
    /// Efficiently inserts multiple reactions in a single query.
    pub async fn bulk_add_reactions(
        &self,
        reactions: &[(i64, i64, &str)], // (message_id, user_id, emoji)
    ) -> Result<(), AppError> {
        if reactions.is_empty() {
            return Ok(());
        }

        // Build bulk insert query
        let mut query = String::from(
            "INSERT INTO message_reactions (message_id, user_id, emoji) VALUES ",
        );
        let mut params: Vec<String> = Vec::with_capacity(reactions.len());

        for (i, _) in reactions.iter().enumerate() {
            let base = i * 3;
            params.push(format!("(${}, ${}, ${})", base + 1, base + 2, base + 3));
        }

        query.push_str(&params.join(", "));
        query.push_str(" ON CONFLICT (message_id, user_id, emoji) DO NOTHING");

        let mut q = sqlx::query(&query);
        for (message_id, user_id, emoji) in reactions {
            q = q.bind(message_id).bind(user_id).bind(*emoji);
        }

        q.execute(&self.pool).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reaction_group_creation() {
        let group = ReactionGroup {
            emoji: "thumbsup".to_string(),
            count: 5,
            first_reaction_at: Utc::now(),
        };

        assert_eq!(group.emoji, "thumbsup");
        assert_eq!(group.count, 5);
    }
}
