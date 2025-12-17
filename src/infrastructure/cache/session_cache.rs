//! Session Cache Service
//!
//! Redis-based caching for user sessions and presence.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;
use super::keys;

/// Cached session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub user_id: i64,
    pub session_id: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

/// User presence data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: i64,
    pub status: String,
    pub custom_status: Option<String>,
    pub last_seen: i64,
    pub guild_ids: Vec<i64>,
}

/// Session cache service for managing user sessions and presence
#[derive(Clone)]
pub struct SessionCacheService {
    redis: ConnectionManager,
    session_ttl: u64,
    presence_ttl: u64,
}

impl SessionCacheService {
    /// Create a new session cache service
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            session_ttl: 7 * 24 * 60 * 60, // 7 days for sessions
            presence_ttl: 5 * 60,           // 5 minutes for presence
        }
    }

    /// Create with custom TTLs
    pub fn with_ttl(redis: ConnectionManager, session_ttl: u64, presence_ttl: u64) -> Self {
        Self {
            redis,
            session_ttl,
            presence_ttl,
        }
    }

    // --- Session Methods ---

    /// Cache a user session
    pub async fn set_session(&self, token_hash: &str, session: &CachedSession) -> Result<(), AppError> {
        let key = format!("{}{}", keys::USER_SESSION, token_hash);
        let value = serde_json::to_string(session)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.session_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get a cached session by token hash
    pub async fn get_session(&self, token_hash: &str) -> Result<Option<CachedSession>, AppError> {
        let key = format!("{}{}", keys::USER_SESSION, token_hash);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let session = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    /// Delete a cached session
    pub async fn delete_session(&self, token_hash: &str) -> Result<bool, AppError> {
        let key = format!("{}{}", keys::USER_SESSION, token_hash);

        let mut conn = self.redis.clone();
        let deleted: i64 = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(deleted > 0)
    }

    /// Check if a session exists
    pub async fn session_exists(&self, token_hash: &str) -> Result<bool, AppError> {
        let key = format!("{}{}", keys::USER_SESSION, token_hash);

        let mut conn = self.redis.clone();
        let exists: bool = conn
            .exists(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(exists)
    }

    /// Refresh session TTL
    pub async fn refresh_session(&self, token_hash: &str) -> Result<bool, AppError> {
        let key = format!("{}{}", keys::USER_SESSION, token_hash);

        let mut conn = self.redis.clone();
        let result: bool = conn
            .expire(&key, self.session_ttl as i64)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(result)
    }

    // --- Presence Methods ---

    /// Set user presence
    pub async fn set_presence(&self, user_id: i64, presence: &UserPresence) -> Result<(), AppError> {
        let key = format!("{}{}", keys::USER_PRESENCE, user_id);
        let value = serde_json::to_string(presence)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.presence_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get user presence
    pub async fn get_presence(&self, user_id: i64) -> Result<Option<UserPresence>, AppError> {
        let key = format!("{}{}", keys::USER_PRESENCE, user_id);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let presence = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(presence))
            }
            None => Ok(None),
        }
    }

    /// Get multiple user presences
    pub async fn get_presences(&self, user_ids: &[i64]) -> Result<Vec<(i64, UserPresence)>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let keys: Vec<String> = user_ids
            .iter()
            .map(|id| format!("{}{}", keys::USER_PRESENCE, id))
            .collect();

        let mut conn = self.redis.clone();
        let values: Vec<Option<String>> = redis::cmd("MGET")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        let mut results = Vec::new();
        for (user_id, value) in user_ids.iter().zip(values.into_iter()) {
            if let Some(json) = value {
                if let Ok(presence) = serde_json::from_str(&json) {
                    results.push((*user_id, presence));
                }
            }
        }

        Ok(results)
    }

    /// Delete user presence
    pub async fn delete_presence(&self, user_id: i64) -> Result<bool, AppError> {
        let key = format!("{}{}", keys::USER_PRESENCE, user_id);

        let mut conn = self.redis.clone();
        let deleted: i64 = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(deleted > 0)
    }

    /// Update presence status only (optimized)
    pub async fn update_status(&self, user_id: i64, status: &str) -> Result<bool, AppError> {
        // Get existing presence and update status
        if let Some(mut presence) = self.get_presence(user_id).await? {
            presence.status = status.to_string();
            presence.last_seen = chrono::Utc::now().timestamp();
            self.set_presence(user_id, &presence).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Heartbeat - update last_seen timestamp
    pub async fn heartbeat(&self, user_id: i64) -> Result<bool, AppError> {
        let key = format!("{}{}", keys::USER_PRESENCE, user_id);

        // First check if presence exists
        let mut conn = self.redis.clone();
        let exists: bool = conn
            .exists(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        if exists {
            // Refresh TTL
            let _: bool = conn
                .expire(&key, self.presence_ttl as i64)
                .await
                .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
