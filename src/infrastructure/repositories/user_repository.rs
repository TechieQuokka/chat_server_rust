//! User Repository Implementation
//!
//! PostgreSQL implementation of the UserRepository trait.
//! Maps between the database schema and domain User entity.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{User, UserRepository, UserStatus};
use crate::shared::error::AppError;

/// Database row representation matching the actual users table schema.
/// This is used internally for sqlx queries since the domain User has more fields.
#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    status: Option<String>,
    bio: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl UserRow {
    /// Convert database row to domain User entity.
    fn into_user(self) -> User {
        User {
            id: self.id,
            username: self.username,
            email: self.email,
            password_hash: self.password_hash,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            status: self.status.map(|s| UserStatus::from_str(&s)).unwrap_or_default(),
            bio: self.bio,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// PostgreSQL user repository implementation.
///
/// Provides CRUD operations for users against a PostgreSQL database.
/// Uses sqlx for compile-time verified queries.
#[derive(Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// Create a new PgUserRepository with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    /// Find a user by their internal ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, display_name, avatar_url,
                   status, bio, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_user()))
    }

    /// Find a user by their email address.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, display_name, avatar_url,
                   status, bio, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_user()))
    }

    /// Find a user by username.
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, display_name, avatar_url,
                   status, bio, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_user()))
    }

    /// Create a new user in the database.
    async fn create(&self, user: &User) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (id, username, email, password_hash, display_name, avatar_url, status, bio)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, username, email, password_hash, display_name, avatar_url,
                      status, bio, created_at, updated_at
            "#,
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(user.status.as_str())
        .bind(&user.bio)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("User with this email or username already exists".to_string())
            }
            _ => AppError::Database(e),
        })?;

        Ok(row.into_user())
    }

    /// Update an existing user.
    async fn update(&self, user: &User) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET username = $2,
                display_name = $3,
                avatar_url = $4,
                bio = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, username, email, password_hash, display_name, avatar_url,
                      status, bio, created_at, updated_at
            "#,
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.bio)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", user.id)))?;

        Ok(row.into_user())
    }

    /// Delete a user (hard delete).
    /// Note: Consider implementing soft delete by adding deleted_at column.
    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User with id {} not found", id)));
        }

        Ok(())
    }

    /// Check if an email address is already registered.
    async fn email_exists(&self, email: &str) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Check if a username is taken.
    async fn username_exists(&self, username: &str) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Update user's online status.
    async fn update_status(&self, id: i64, status: UserStatus) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE users SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status.as_str())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User with id {} not found", id)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here, requiring a test database
}
