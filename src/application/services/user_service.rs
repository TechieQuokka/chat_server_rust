//! User Service
//!
//! Handles user management operations.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{Server, ServerRepository, User, UserRepository, UserStatus};

/// User service trait
#[async_trait]
pub trait UserService: Send + Sync {
    /// Get user by ID
    async fn get_user(&self, user_id: i64) -> Result<UserDto, UserError>;

    /// Get user by username
    async fn get_user_by_username(&self, username: &str) -> Result<UserDto, UserError>;

    /// Update user profile
    async fn update_profile(&self, user_id: i64, update: UpdateProfileDto) -> Result<UserDto, UserError>;

    /// Update user status
    async fn update_status(&self, user_id: i64, status: &str) -> Result<(), UserError>;

    /// Get user's servers (guilds)
    async fn get_user_servers(&self, user_id: i64) -> Result<Vec<ServerPreviewDto>, UserError>;

    /// Delete user account
    async fn delete_user(&self, user_id: i64) -> Result<(), UserError>;
}

/// User data transfer object
#[derive(Debug, Clone)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub bio: Option<String>,
    pub created_at: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            status: user.status.as_str().to_string(),
            bio: user.bio,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

/// Update profile request
#[derive(Debug, Clone, Default)]
pub struct UpdateProfileDto {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

/// Server preview for user's server list
#[derive(Debug, Clone)]
pub struct ServerPreviewDto {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner: bool,
}

impl ServerPreviewDto {
    pub fn from_server(server: Server, user_id: i64) -> Self {
        Self {
            id: server.id.to_string(),
            name: server.name,
            icon_url: server.icon_url,
            owner: server.owner_id == user_id,
        }
    }
}

/// User service errors
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("User not found")]
    NotFound,

    #[error("Username already taken")]
    UsernameTaken,

    #[error("Invalid status")]
    InvalidStatus,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// UserService implementation
pub struct UserServiceImpl<U, S>
where
    U: UserRepository,
    S: ServerRepository,
{
    user_repo: Arc<U>,
    server_repo: Arc<S>,
}

impl<U, S> UserServiceImpl<U, S>
where
    U: UserRepository,
    S: ServerRepository,
{
    pub fn new(user_repo: Arc<U>, server_repo: Arc<S>) -> Self {
        Self {
            user_repo,
            server_repo,
        }
    }
}

#[async_trait]
impl<U, S> UserService for UserServiceImpl<U, S>
where
    U: UserRepository + 'static,
    S: ServerRepository + 'static,
{
    async fn get_user(&self, user_id: i64) -> Result<UserDto, UserError> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?
            .ok_or(UserError::NotFound)?;

        Ok(UserDto::from(user))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<UserDto, UserError> {
        let user = self
            .user_repo
            .find_by_username(username)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?
            .ok_or(UserError::NotFound)?;

        Ok(UserDto::from(user))
    }

    async fn update_profile(&self, user_id: i64, update: UpdateProfileDto) -> Result<UserDto, UserError> {
        // Get existing user
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?
            .ok_or(UserError::NotFound)?;

        // Check if username is being changed and if it's available
        if let Some(ref new_username) = update.username {
            if new_username != &user.username {
                let exists = self
                    .user_repo
                    .username_exists(new_username)
                    .await
                    .map_err(|e| UserError::Internal(e.to_string()))?;

                if exists {
                    return Err(UserError::UsernameTaken);
                }
                user.username = new_username.clone();
            }
        }

        // Apply updates
        if let Some(display_name) = update.display_name {
            user.display_name = Some(display_name);
        }
        if let Some(avatar_url) = update.avatar_url {
            user.avatar_url = Some(avatar_url);
        }
        if let Some(bio) = update.bio {
            user.bio = Some(bio);
        }

        // Save updates
        let updated = self
            .user_repo
            .update(&user)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        Ok(UserDto::from(updated))
    }

    async fn update_status(&self, user_id: i64, status: &str) -> Result<(), UserError> {
        let status = match status.to_lowercase().as_str() {
            "online" => UserStatus::Online,
            "idle" => UserStatus::Idle,
            "dnd" => UserStatus::Dnd,
            "invisible" | "offline" => UserStatus::Offline,
            _ => return Err(UserError::InvalidStatus),
        };

        self.user_repo
            .update_status(user_id, status)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_user_servers(&self, user_id: i64) -> Result<Vec<ServerPreviewDto>, UserError> {
        let servers = self
            .server_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        Ok(servers
            .into_iter()
            .map(|s| ServerPreviewDto::from_server(s, user_id))
            .collect())
    }

    async fn delete_user(&self, user_id: i64) -> Result<(), UserError> {
        self.user_repo
            .delete(user_id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here
}
