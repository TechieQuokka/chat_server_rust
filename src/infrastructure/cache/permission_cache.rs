//! Permission Cache Service
//!
//! Redis-based caching for computed permissions.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Cache key prefixes for permission caching
mod keys {
    pub const MEMBER_PERMS: &str = "perms:member:";
    pub const CHANNEL_PERMS: &str = "perms:channel:";
    pub const GUILD_MEMBERS: &str = "guild:members:";
}

/// Cached member permissions for a guild
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMemberPermissions {
    pub user_id: i64,
    pub guild_id: i64,
    pub permissions: u64,
    pub role_ids: Vec<i64>,
    pub is_owner: bool,
}

/// Cached channel permissions for a member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedChannelPermissions {
    pub user_id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub permissions: u64,
    pub can_view: bool,
    pub can_send: bool,
    pub can_manage: bool,
}

/// Guild member info for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGuildMember {
    pub user_id: i64,
    pub nickname: Option<String>,
    pub role_ids: Vec<i64>,
    pub joined_at: i64,
}

/// Permission cache service
#[derive(Clone)]
pub struct PermissionCacheService {
    redis: ConnectionManager,
    member_perms_ttl: u64,
    channel_perms_ttl: u64,
    guild_members_ttl: u64,
}

impl PermissionCacheService {
    /// Create a new permission cache service
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            member_perms_ttl: 5 * 60,     // 5 minutes for member permissions
            channel_perms_ttl: 5 * 60,    // 5 minutes for channel permissions
            guild_members_ttl: 10 * 60,   // 10 minutes for guild members list
        }
    }

    /// Create with custom TTLs
    pub fn with_ttl(
        redis: ConnectionManager,
        member_perms_ttl: u64,
        channel_perms_ttl: u64,
        guild_members_ttl: u64,
    ) -> Self {
        Self {
            redis,
            member_perms_ttl,
            channel_perms_ttl,
            guild_members_ttl,
        }
    }

    // --- Member Permissions ---

    /// Cache member permissions for a guild
    pub async fn set_member_permissions(
        &self,
        guild_id: i64,
        user_id: i64,
        perms: &CachedMemberPermissions,
    ) -> Result<(), AppError> {
        let key = format!("{}{}:{}", keys::MEMBER_PERMS, guild_id, user_id);
        let value = serde_json::to_string(perms)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.member_perms_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get cached member permissions
    pub async fn get_member_permissions(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<Option<CachedMemberPermissions>, AppError> {
        let key = format!("{}{}:{}", keys::MEMBER_PERMS, guild_id, user_id);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let perms = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(perms))
            }
            None => Ok(None),
        }
    }

    /// Invalidate member permissions for a guild
    pub async fn invalidate_member_permissions(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<bool, AppError> {
        let key = format!("{}{}:{}", keys::MEMBER_PERMS, guild_id, user_id);

        let mut conn = self.redis.clone();
        let deleted: i64 = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(deleted > 0)
    }

    /// Invalidate all permissions for a user in a guild (e.g., when roles change)
    pub async fn invalidate_user_guild_permissions(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        // Delete member permissions
        let member_key = format!("{}{}:{}", keys::MEMBER_PERMS, guild_id, user_id);

        let mut conn = self.redis.clone();
        let _: () = conn
            .del(&member_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Note: Channel permissions would need to be invalidated separately
        // if we track which channels the user has cached permissions for

        Ok(())
    }

    // --- Channel Permissions ---

    /// Cache channel permissions for a member
    pub async fn set_channel_permissions(
        &self,
        channel_id: i64,
        user_id: i64,
        perms: &CachedChannelPermissions,
    ) -> Result<(), AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_PERMS, channel_id, user_id);
        let value = serde_json::to_string(perms)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.channel_perms_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get cached channel permissions
    pub async fn get_channel_permissions(
        &self,
        channel_id: i64,
        user_id: i64,
    ) -> Result<Option<CachedChannelPermissions>, AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_PERMS, channel_id, user_id);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let perms = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(perms))
            }
            None => Ok(None),
        }
    }

    /// Invalidate channel permissions for a user
    pub async fn invalidate_channel_permissions(
        &self,
        channel_id: i64,
        user_id: i64,
    ) -> Result<bool, AppError> {
        let key = format!("{}{}:{}", keys::CHANNEL_PERMS, channel_id, user_id);

        let mut conn = self.redis.clone();
        let deleted: i64 = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(deleted > 0)
    }

    // --- Guild Members Cache ---

    /// Cache guild member list (IDs only for efficiency)
    pub async fn set_guild_member_ids(
        &self,
        guild_id: i64,
        member_ids: &[i64],
    ) -> Result<(), AppError> {
        let key = format!("{}{}:ids", keys::GUILD_MEMBERS, guild_id);
        let value = serde_json::to_string(member_ids)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.guild_members_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get cached guild member IDs
    pub async fn get_guild_member_ids(&self, guild_id: i64) -> Result<Option<Vec<i64>>, AppError> {
        let key = format!("{}{}:ids", keys::GUILD_MEMBERS, guild_id);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let ids = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(ids))
            }
            None => Ok(None),
        }
    }

    /// Cache a single guild member
    pub async fn set_guild_member(
        &self,
        guild_id: i64,
        member: &CachedGuildMember,
    ) -> Result<(), AppError> {
        let key = format!("{}{}:{}", keys::GUILD_MEMBERS, guild_id, member.user_id);
        let value = serde_json::to_string(member)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, value, self.guild_members_ttl)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Get a cached guild member
    pub async fn get_guild_member(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<Option<CachedGuildMember>, AppError> {
        let key = format!("{}{}:{}", keys::GUILD_MEMBERS, guild_id, user_id);

        let mut conn = self.redis.clone();
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match value {
            Some(json) => {
                let member = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(member))
            }
            None => Ok(None),
        }
    }

    /// Invalidate guild member cache
    pub async fn invalidate_guild_member(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        let member_key = format!("{}{}:{}", keys::GUILD_MEMBERS, guild_id, user_id);
        let ids_key = format!("{}{}:ids", keys::GUILD_MEMBERS, guild_id);

        let mut conn = self.redis.clone();

        // Delete individual member cache
        let _: () = conn
            .del(&member_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Also invalidate the member IDs list since it's now stale
        let _: () = conn
            .del(&ids_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Invalidate all caches for a guild (e.g., when guild is deleted)
    pub async fn invalidate_guild(&self, guild_id: i64) -> Result<(), AppError> {
        // Delete member IDs list
        let ids_key = format!("{}{}:ids", keys::GUILD_MEMBERS, guild_id);

        let mut conn = self.redis.clone();
        let _: () = conn
            .del(&ids_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Note: Individual member caches and permission caches would need
        // pattern-based deletion which requires SCAN command
        // For production, consider using Redis keyspace notifications or
        // a separate data structure to track keys per guild

        Ok(())
    }

    /// Check if user is a member of guild (cache-first)
    pub async fn is_guild_member(&self, guild_id: i64, user_id: i64) -> Result<Option<bool>, AppError> {
        // First try member IDs list
        if let Some(ids) = self.get_guild_member_ids(guild_id).await? {
            return Ok(Some(ids.contains(&user_id)));
        }

        // If no IDs list, try individual member cache
        if self.get_guild_member(guild_id, user_id).await?.is_some() {
            return Ok(Some(true));
        }

        // Cache miss - caller should check database
        Ok(None)
    }
}
