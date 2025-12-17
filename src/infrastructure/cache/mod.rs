//! Cache Module
//!
//! Redis connection management and caching utilities.
//!
//! This module provides:
//! - Redis connection management with automatic reconnection
//! - A generic `Cache` trait for abstracting cache operations
//! - A `RedisCache` implementation with full Redis support
//! - Predefined key prefixes for consistent cache key naming
//!
//! # Architecture
//!
//! ```text
//! +-------------------+
//! |   Application     |
//! +-------------------+
//!          |
//!          v
//! +-------------------+
//! |   Cache Trait     |  <-- Abstract interface
//! +-------------------+
//!          |
//!          v
//! +-------------------+
//! |   RedisCache      |  <-- Concrete implementation
//! +-------------------+
//!          |
//!          v
//! +-------------------+
//! | ConnectionManager |  <-- Redis connection pool
//! +-------------------+
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use chat_server::infrastructure::cache::{Cache, RedisCache, create_redis_client};
//! use chat_server::config::RedisSettings;
//!
//! // Create connection
//! let settings = RedisSettings { url: "redis://localhost:6379".into() };
//! let conn = create_redis_client(&settings).await?;
//!
//! // Create cache instance
//! let cache = RedisCache::new(conn);
//!
//! // Use the cache
//! cache.set_ex("user:123", &user_data, 3600).await?;
//! let user: Option<UserData> = cache.get("user:123").await?;
//! ```

mod cache_service;
mod permission_cache;
mod session_cache;
mod typing_cache;

pub use cache_service::{Cache, RedisCache};
pub use permission_cache::{
    CachedChannelPermissions, CachedGuildMember, CachedMemberPermissions, PermissionCacheService,
};
pub use session_cache::{CachedSession, SessionCacheService, UserPresence};
pub use typing_cache::TypingCacheService;

use redis::aio::ConnectionManager;
use redis::Client;
use tracing::{info, instrument};

use crate::config::RedisSettings;

/// Creates a Redis connection manager with automatic reconnection.
///
/// The connection manager handles connection pooling and automatic
/// reconnection when the connection is lost.
///
/// # Arguments
/// * `settings` - Redis configuration settings
///
/// # Returns
/// * `Ok(ConnectionManager)` - On successful connection
/// * `Err(redis::RedisError)` - If connection fails
///
/// # Example
/// ```rust,ignore
/// let settings = RedisSettings { url: "redis://localhost:6379".into() };
/// let conn = create_redis_client(&settings).await?;
/// ```
#[instrument(skip(settings), fields(url = %settings.url))]
pub async fn create_redis_client(
    settings: &RedisSettings,
) -> Result<ConnectionManager, redis::RedisError> {
    info!("Connecting to Redis...");
    let client = Client::open(settings.url.as_str())?;
    let manager = ConnectionManager::new(client).await?;
    info!("Redis connection established");
    Ok(manager)
}

/// Creates a `RedisCache` instance from configuration settings.
///
/// This is a convenience function that creates both the connection
/// manager and the cache instance in one call.
///
/// # Arguments
/// * `settings` - Redis configuration settings
///
/// # Returns
/// * `Ok(RedisCache)` - On successful connection
/// * `Err(redis::RedisError)` - If connection fails
#[instrument(skip(settings), fields(url = %settings.url))]
pub async fn create_redis_cache(settings: &RedisSettings) -> Result<RedisCache, redis::RedisError> {
    let conn = create_redis_client(settings).await?;
    Ok(RedisCache::new(conn))
}

/// Creates a `RedisCache` instance with a key prefix.
///
/// Useful for multi-tenant scenarios or logical separation of data.
///
/// # Arguments
/// * `settings` - Redis configuration settings
/// * `prefix` - Prefix to prepend to all cache keys
///
/// # Returns
/// * `Ok(RedisCache)` - On successful connection
/// * `Err(redis::RedisError)` - If connection fails
#[instrument(skip(settings), fields(url = %settings.url, prefix = %prefix))]
pub async fn create_redis_cache_with_prefix(
    settings: &RedisSettings,
    prefix: &str,
) -> Result<RedisCache, redis::RedisError> {
    let conn = create_redis_client(settings).await?;
    Ok(RedisCache::with_prefix(conn, prefix))
}

/// Cache key prefixes for different data types.
///
/// Use these constants to ensure consistent key naming across the application.
///
/// # Example
/// ```rust,ignore
/// use chat_server::infrastructure::cache::keys;
///
/// let session_key = format!("{}{}", keys::USER_SESSION, user_id);
/// ```
pub mod keys {
    /// Prefix for user session data (e.g., "session:user_id")
    pub const USER_SESSION: &str = "session:";

    /// Prefix for user presence/online status (e.g., "presence:user_id")
    pub const USER_PRESENCE: &str = "presence:";

    /// Prefix for guild member lists (e.g., "guild:members:guild_id")
    pub const GUILD_MEMBERS: &str = "guild:members:";

    /// Prefix for channel typing indicators (e.g., "channel:typing:channel_id")
    pub const CHANNEL_TYPING: &str = "channel:typing:";

    /// Prefix for rate limiting counters (e.g., "ratelimit:user_id:action")
    pub const RATE_LIMIT: &str = "ratelimit:";

    /// Prefix for user profile cache (e.g., "user:user_id")
    pub const USER_PROFILE: &str = "user:";

    /// Prefix for channel data cache (e.g., "channel:channel_id")
    pub const CHANNEL: &str = "channel:";

    /// Prefix for message data cache (e.g., "message:message_id")
    pub const MESSAGE: &str = "message:";

    /// Prefix for guild data cache (e.g., "guild:guild_id")
    pub const GUILD: &str = "guild:";

    /// Prefix for permission cache (e.g., "perms:user_id:resource")
    pub const PERMISSIONS: &str = "perms:";

    /// Prefix for distributed locks (e.g., "lock:resource_name")
    pub const LOCK: &str = "lock:";

    /// Generates a session key for a user
    #[inline]
    pub fn session(user_id: impl std::fmt::Display) -> String {
        format!("{}{}", USER_SESSION, user_id)
    }

    /// Generates a presence key for a user
    #[inline]
    pub fn presence(user_id: impl std::fmt::Display) -> String {
        format!("{}{}", USER_PRESENCE, user_id)
    }

    /// Generates a guild members key
    #[inline]
    pub fn guild_members(guild_id: impl std::fmt::Display) -> String {
        format!("{}{}", GUILD_MEMBERS, guild_id)
    }

    /// Generates a typing indicator key
    #[inline]
    pub fn typing(channel_id: impl std::fmt::Display, user_id: impl std::fmt::Display) -> String {
        format!("{}{}:{}", CHANNEL_TYPING, channel_id, user_id)
    }

    /// Generates a rate limit key
    #[inline]
    pub fn rate_limit(user_id: impl std::fmt::Display, action: &str) -> String {
        format!("{}{}:{}", RATE_LIMIT, user_id, action)
    }

    /// Generates a user profile cache key
    #[inline]
    pub fn user(user_id: impl std::fmt::Display) -> String {
        format!("{}{}", USER_PROFILE, user_id)
    }

    /// Generates a channel cache key
    #[inline]
    pub fn channel(channel_id: impl std::fmt::Display) -> String {
        format!("{}{}", CHANNEL, channel_id)
    }

    /// Generates a message cache key
    #[inline]
    pub fn message(message_id: impl std::fmt::Display) -> String {
        format!("{}{}", MESSAGE, message_id)
    }

    /// Generates a guild cache key
    #[inline]
    pub fn guild(guild_id: impl std::fmt::Display) -> String {
        format!("{}{}", GUILD, guild_id)
    }

    /// Generates a permission cache key
    #[inline]
    pub fn permissions(user_id: impl std::fmt::Display, resource: &str) -> String {
        format!("{}{}:{}", PERMISSIONS, user_id, resource)
    }

    /// Generates a distributed lock key
    #[inline]
    pub fn lock(resource: &str) -> String {
        format!("{}{}", LOCK, resource)
    }
}
