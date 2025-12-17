//! Typing Indicator Cache
//!
//! Redis-based caching for typing indicators in channels.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::shared::error::AppError;
use super::keys;

/// Typing indicator cache service
#[derive(Clone)]
pub struct TypingCacheService {
    redis: ConnectionManager,
    typing_ttl: u64,
}

impl TypingCacheService {
    /// Create a new typing cache service
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            typing_ttl: 10, // 10 seconds (Discord standard)
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(redis: ConnectionManager, typing_ttl: u64) -> Self {
        Self { redis, typing_ttl }
    }

    /// Set user as typing in a channel
    pub async fn set_typing(&self, channel_id: i64, user_id: i64) -> Result<(), AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_TYPING, channel_id, user_id);
        let timestamp = chrono::Utc::now().timestamp();

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, timestamp, self.typing_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Also add to channel's typing set for efficient lookups
        let set_key = format!("{}{}:users", keys::CHANNEL_TYPING, channel_id);
        conn.sadd::<_, _, ()>(&set_key, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Refresh set TTL
        conn.expire::<_, ()>(&set_key, self.typing_ttl as i64)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Check if user is currently typing in a channel
    pub async fn is_typing(&self, channel_id: i64, user_id: i64) -> Result<bool, AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_TYPING, channel_id, user_id);

        let mut conn = self.redis.clone();
        let exists: bool = conn
            .exists(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(exists)
    }

    /// Get all users currently typing in a channel
    pub async fn get_typing_users(&self, channel_id: i64) -> Result<Vec<i64>, AppError> {
        let set_key = format!("{}{}:users", keys::CHANNEL_TYPING, channel_id);

        let mut conn = self.redis.clone();
        let members: Vec<i64> = conn
            .smembers(&set_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Filter out expired entries by checking individual keys
        let mut active_users = Vec::new();
        for user_id in members {
            if self.is_typing(channel_id, user_id).await? {
                active_users.push(user_id);
            } else {
                // Clean up stale entry from set
                let _: () = conn
                    .srem(&set_key, user_id)
                    .await
                    .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;
            }
        }

        Ok(active_users)
    }

    /// Clear typing indicator for a user in a channel
    pub async fn clear_typing(&self, channel_id: i64, user_id: i64) -> Result<(), AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_TYPING, channel_id, user_id);
        let set_key = format!("{}{}:users", keys::CHANNEL_TYPING, channel_id);

        let mut conn = self.redis.clone();

        // Remove individual key
        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Remove from set
        let _: () = conn
            .srem(&set_key, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Clear all typing indicators in a channel
    pub async fn clear_channel(&self, channel_id: i64) -> Result<(), AppError> {
        let set_key = format!("{}{}:users", keys::CHANNEL_TYPING, channel_id);

        let mut conn = self.redis.clone();

        // Get all users in the set
        let members: Vec<i64> = conn
            .smembers(&set_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Delete all individual keys
        for user_id in members {
            let key = format!("{}{}:{}", keys::CHANNEL_TYPING, channel_id, user_id);
            let _: () = conn
                .del(&key)
                .await
                .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;
        }

        // Delete the set
        let _: () = conn
            .del(&set_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get typing indicator timestamp for a user
    pub async fn get_typing_timestamp(
        &self,
        channel_id: i64,
        user_id: i64,
    ) -> Result<Option<i64>, AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_TYPING, channel_id, user_id);

        let mut conn = self.redis.clone();
        let timestamp: Option<i64> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(timestamp)
    }
}
