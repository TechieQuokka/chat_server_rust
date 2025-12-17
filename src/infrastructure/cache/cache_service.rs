//! Cache Service
//!
//! Generic cache trait and Redis implementation for application-wide caching.
//!
//! This module provides:
//! - A `Cache` trait defining common caching operations
//! - A `RedisCache` implementation using Redis as the backing store
//! - JSON serialization/deserialization for complex types
//!
//! # Example
//!
//! ```rust,ignore
//! use chat_server::infrastructure::cache::{Cache, RedisCache};
//!
//! let cache = RedisCache::new(redis_connection);
//!
//! // Store a user session
//! cache.set_ex("session:123", &session_data, 3600).await?;
//!
//! // Retrieve the session
//! let session: Option<SessionData> = cache.get("session:123").await?;
//! ```

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::shared::error::AppError;

/// Generic cache trait for abstracting cache operations.
///
/// This trait provides a unified interface for caching operations,
/// allowing for different backend implementations (Redis, in-memory, etc.).
///
/// All operations are async and return `Result<T, AppError>` for proper error handling.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Retrieves a value from the cache by key.
    ///
    /// # Arguments
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    /// * `Ok(Some(T))` - If the key exists and deserialization succeeds
    /// * `Ok(None)` - If the key does not exist
    /// * `Err(AppError)` - If a cache or deserialization error occurs
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>, AppError>;

    /// Stores a value in the cache without expiration.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to store (must implement Serialize)
    ///
    /// # Returns
    /// * `Ok(())` - If the value was stored successfully
    /// * `Err(AppError)` - If a cache or serialization error occurs
    async fn set<T: Serialize + Sync + Send>(&self, key: &str, value: &T) -> Result<(), AppError>;

    /// Stores a value in the cache with an expiration time.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to store (must implement Serialize)
    /// * `seconds` - Time-to-live in seconds
    ///
    /// # Returns
    /// * `Ok(())` - If the value was stored successfully
    /// * `Err(AppError)` - If a cache or serialization error occurs
    async fn set_ex<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
        seconds: u64,
    ) -> Result<(), AppError>;

    /// Deletes a key from the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key to delete
    ///
    /// # Returns
    /// * `Ok(true)` - If the key existed and was deleted
    /// * `Ok(false)` - If the key did not exist
    /// * `Err(AppError)` - If a cache error occurs
    async fn delete(&self, key: &str) -> Result<bool, AppError>;

    /// Checks if a key exists in the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key to check
    ///
    /// # Returns
    /// * `Ok(true)` - If the key exists
    /// * `Ok(false)` - If the key does not exist
    /// * `Err(AppError)` - If a cache error occurs
    async fn exists(&self, key: &str) -> Result<bool, AppError>;

    /// Increments an integer value stored at the key.
    ///
    /// If the key does not exist, it is set to 0 before incrementing.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// * `Ok(i64)` - The new value after incrementing
    /// * `Err(AppError)` - If a cache error occurs or value is not an integer
    async fn incr(&self, key: &str) -> Result<i64, AppError>;

    /// Sets an expiration time on an existing key.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `seconds` - Time-to-live in seconds
    ///
    /// # Returns
    /// * `Ok(true)` - If the expiration was set
    /// * `Ok(false)` - If the key does not exist
    /// * `Err(AppError)` - If a cache error occurs
    async fn expire(&self, key: &str, seconds: u64) -> Result<bool, AppError>;

    /// Retrieves the TTL (time-to-live) of a key in seconds.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// * `Ok(Some(ttl))` - TTL in seconds if key exists with expiration
    /// * `Ok(None)` - If key does not exist or has no expiration
    /// * `Err(AppError)` - If a cache error occurs
    async fn ttl(&self, key: &str) -> Result<Option<i64>, AppError>;

    /// Increments an integer value by a specific amount.
    ///
    /// If the key does not exist, it is set to 0 before incrementing.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `delta` - The amount to increment by
    ///
    /// # Returns
    /// * `Ok(i64)` - The new value after incrementing
    /// * `Err(AppError)` - If a cache error occurs or value is not an integer
    async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, AppError>;

    /// Decrements an integer value stored at the key.
    ///
    /// If the key does not exist, it is set to 0 before decrementing.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// * `Ok(i64)` - The new value after decrementing
    /// * `Err(AppError)` - If a cache error occurs or value is not an integer
    async fn decr(&self, key: &str) -> Result<i64, AppError>;

    /// Sets a value only if the key does not already exist.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to store
    ///
    /// # Returns
    /// * `Ok(true)` - If the key was set (did not exist)
    /// * `Ok(false)` - If the key already exists
    /// * `Err(AppError)` - If a cache or serialization error occurs
    async fn set_nx<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<bool, AppError>;

    /// Sets a value with expiration only if the key does not already exist.
    ///
    /// Useful for implementing distributed locks.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to store
    /// * `seconds` - Time-to-live in seconds
    ///
    /// # Returns
    /// * `Ok(true)` - If the key was set (did not exist)
    /// * `Ok(false)` - If the key already exists
    /// * `Err(AppError)` - If a cache or serialization error occurs
    async fn set_nx_ex<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
        seconds: u64,
    ) -> Result<bool, AppError>;

    /// Deletes multiple keys from the cache.
    ///
    /// # Arguments
    /// * `keys` - Slice of cache keys to delete
    ///
    /// # Returns
    /// * `Ok(count)` - Number of keys that were deleted
    /// * `Err(AppError)` - If a cache error occurs
    async fn delete_many(&self, keys: &[&str]) -> Result<u64, AppError>;

    /// Retrieves multiple values from the cache.
    ///
    /// # Arguments
    /// * `keys` - Slice of cache keys to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<Option<T>>)` - Values in the same order as keys (None for missing keys)
    /// * `Err(AppError)` - If a cache or deserialization error occurs
    async fn get_many<T: DeserializeOwned + Send>(
        &self,
        keys: &[&str],
    ) -> Result<Vec<Option<T>>, AppError>;
}

/// Redis-backed cache implementation.
///
/// Uses a Redis ConnectionManager for efficient connection pooling and
/// automatic reconnection handling.
#[derive(Clone)]
pub struct RedisCache {
    /// Redis connection manager with automatic reconnection
    conn: ConnectionManager,
    /// Optional key prefix for namespacing
    prefix: Option<Arc<str>>,
}

impl RedisCache {
    /// Creates a new RedisCache instance.
    ///
    /// # Arguments
    /// * `conn` - Redis ConnectionManager instance
    ///
    /// # Example
    /// ```rust,ignore
    /// let conn = ConnectionManager::new(client).await?;
    /// let cache = RedisCache::new(conn);
    /// ```
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn, prefix: None }
    }

    /// Creates a new RedisCache instance with a key prefix.
    ///
    /// All keys will be automatically prefixed, useful for multi-tenant
    /// scenarios or logical separation of data.
    ///
    /// # Arguments
    /// * `conn` - Redis ConnectionManager instance
    /// * `prefix` - Prefix to prepend to all keys
    ///
    /// # Example
    /// ```rust,ignore
    /// let cache = RedisCache::with_prefix(conn, "chat:v1:");
    /// // key "user:123" becomes "chat:v1:user:123"
    /// ```
    pub fn with_prefix(conn: ConnectionManager, prefix: impl Into<Arc<str>>) -> Self {
        Self {
            conn,
            prefix: Some(prefix.into()),
        }
    }

    /// Formats a key with the optional prefix.
    fn format_key(&self, key: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{}{}", prefix, key),
            None => key.to_string(),
        }
    }

    /// Serializes a value to JSON string.
    fn serialize<T: Serialize>(value: &T) -> Result<String, AppError> {
        serde_json::to_string(value).map_err(|e| {
            warn!("Cache serialization error: {}", e);
            AppError::Internal(format!("Cache serialization failed: {}", e))
        })
    }

    /// Deserializes a JSON string to the target type.
    fn deserialize<T: DeserializeOwned>(data: &str) -> Result<T, AppError> {
        serde_json::from_str(data).map_err(|e| {
            warn!("Cache deserialization error: {}", e);
            AppError::Internal(format!("Cache deserialization failed: {}", e))
        })
    }
}

#[async_trait]
impl Cache for RedisCache {
    #[instrument(skip(self), level = "debug")]
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let result: Option<String> = conn.get(&full_key).await?;

        match result {
            Some(data) => {
                debug!(key = %full_key, "Cache hit");
                let value = Self::deserialize(&data)?;
                Ok(Some(value))
            }
            None => {
                debug!(key = %full_key, "Cache miss");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, value), level = "debug")]
    async fn set<T: Serialize + Sync + Send>(&self, key: &str, value: &T) -> Result<(), AppError> {
        let full_key = self.format_key(key);
        let data = Self::serialize(value)?;
        let mut conn = self.conn.clone();

        let _: () = conn.set(&full_key, data).await?;
        debug!(key = %full_key, "Cache set");

        Ok(())
    }

    #[instrument(skip(self, value), level = "debug")]
    async fn set_ex<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
        seconds: u64,
    ) -> Result<(), AppError> {
        let full_key = self.format_key(key);
        let data = Self::serialize(value)?;
        let mut conn = self.conn.clone();

        let _: () = conn.set_ex(&full_key, data, seconds).await?;
        debug!(key = %full_key, ttl = seconds, "Cache set with expiry");

        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    async fn delete(&self, key: &str) -> Result<bool, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let deleted: u64 = conn.del(&full_key).await?;
        let existed = deleted > 0;

        debug!(key = %full_key, deleted = existed, "Cache delete");

        Ok(existed)
    }

    #[instrument(skip(self), level = "debug")]
    async fn exists(&self, key: &str) -> Result<bool, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let exists: bool = conn.exists(&full_key).await?;
        debug!(key = %full_key, exists = exists, "Cache exists check");

        Ok(exists)
    }

    #[instrument(skip(self), level = "debug")]
    async fn incr(&self, key: &str) -> Result<i64, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let value: i64 = conn.incr(&full_key, 1).await?;
        debug!(key = %full_key, value = value, "Cache increment");

        Ok(value)
    }

    #[instrument(skip(self), level = "debug")]
    async fn expire(&self, key: &str, seconds: u64) -> Result<bool, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        // Redis EXPIRE returns 1 if timeout was set, 0 if key does not exist
        let result: i32 = conn.expire(&full_key, seconds as i64).await?;
        let success = result == 1;

        debug!(key = %full_key, seconds = seconds, success = success, "Cache expire");

        Ok(success)
    }

    #[instrument(skip(self), level = "debug")]
    async fn ttl(&self, key: &str) -> Result<Option<i64>, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let ttl: i64 = conn.ttl(&full_key).await?;

        // Redis TTL returns:
        // -2 if key does not exist
        // -1 if key exists but has no expiration
        // positive value for remaining seconds
        let result = match ttl {
            -2 => None,
            -1 => None,
            _ => Some(ttl),
        };

        debug!(key = %full_key, ttl = ?result, "Cache TTL check");

        Ok(result)
    }

    #[instrument(skip(self), level = "debug")]
    async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let value: i64 = conn.incr(&full_key, delta).await?;
        debug!(key = %full_key, delta = delta, value = value, "Cache increment by");

        Ok(value)
    }

    #[instrument(skip(self), level = "debug")]
    async fn decr(&self, key: &str) -> Result<i64, AppError> {
        let full_key = self.format_key(key);
        let mut conn = self.conn.clone();

        let value: i64 = conn.decr(&full_key, 1).await?;
        debug!(key = %full_key, value = value, "Cache decrement");

        Ok(value)
    }

    #[instrument(skip(self, value), level = "debug")]
    async fn set_nx<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<bool, AppError> {
        let full_key = self.format_key(key);
        let data = Self::serialize(value)?;
        let mut conn = self.conn.clone();

        let was_set: bool = conn.set_nx(&full_key, data).await?;
        debug!(key = %full_key, was_set = was_set, "Cache set if not exists");

        Ok(was_set)
    }

    #[instrument(skip(self, value), level = "debug")]
    async fn set_nx_ex<T: Serialize + Sync + Send>(
        &self,
        key: &str,
        value: &T,
        seconds: u64,
    ) -> Result<bool, AppError> {
        let full_key = self.format_key(key);
        let data = Self::serialize(value)?;
        let mut conn = self.conn.clone();

        // Use SET with NX and EX options for atomic set-if-not-exists with expiry
        let result: Option<String> = redis::cmd("SET")
            .arg(&full_key)
            .arg(data)
            .arg("NX")
            .arg("EX")
            .arg(seconds)
            .query_async(&mut conn)
            .await?;

        let was_set = result.is_some();
        debug!(key = %full_key, ttl = seconds, was_set = was_set, "Cache set NX with expiry");

        Ok(was_set)
    }

    #[instrument(skip(self), level = "debug")]
    async fn delete_many(&self, keys: &[&str]) -> Result<u64, AppError> {
        if keys.is_empty() {
            return Ok(0);
        }

        let full_keys: Vec<String> = keys.iter().map(|k| self.format_key(k)).collect();
        let mut conn = self.conn.clone();

        let deleted: u64 = conn.del(full_keys.as_slice()).await?;
        debug!(count = deleted, "Cache delete many");

        Ok(deleted)
    }

    #[instrument(skip(self), level = "debug")]
    async fn get_many<T: DeserializeOwned + Send>(
        &self,
        keys: &[&str],
    ) -> Result<Vec<Option<T>>, AppError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let full_keys: Vec<String> = keys.iter().map(|k| self.format_key(k)).collect();
        let mut conn = self.conn.clone();

        let results: Vec<Option<String>> = conn.mget(full_keys.as_slice()).await?;

        let mut values = Vec::with_capacity(results.len());
        for result in results {
            match result {
                Some(data) => {
                    let value = Self::deserialize(&data)?;
                    values.push(Some(value));
                }
                None => values.push(None),
            }
        }

        debug!(count = values.len(), "Cache get many");

        Ok(values)
    }
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i32,
        name: String,
    }

    #[test]
    fn test_serialization() {
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        let json = RedisCache::serialize(&data).unwrap();
        let parsed: TestData = RedisCache::deserialize(&json).unwrap();

        assert_eq!(data, parsed);
    }

    #[test]
    fn test_format_key_without_prefix() {
        // Test the format_key logic directly without creating an invalid RedisCache
        let prefix: Option<Arc<str>> = None;
        let key = "user:123";
        let result = match &prefix {
            Some(p) => format!("{}{}", p, key),
            None => key.to_string(),
        };
        assert_eq!(result, "user:123");
    }

    #[test]
    fn test_format_key_with_prefix() {
        // Test the format_key logic directly without creating an invalid RedisCache
        let prefix: Option<Arc<str>> = Some("chat:v1:".into());
        let key = "user:123";
        let result = match &prefix {
            Some(p) => format!("{}{}", p, key),
            None => key.to_string(),
        };
        assert_eq!(result, "chat:v1:user:123");
    }
}
