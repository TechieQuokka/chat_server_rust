//! Authentication Service
//!
//! Handles user authentication, JWT token management, and session handling.

use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::JwtSettings;
use crate::domain::{Session, SessionRepository, User, UserRepository};
use crate::shared::snowflake::SnowflakeGenerator;

/// Authentication service trait for dependency injection
#[async_trait]
pub trait AuthService: Send + Sync {
    /// Register a new user
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(User, AuthTokens), AuthError>;

    /// Authenticate user with credentials
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthTokens, AuthError>;

    /// Refresh access token using refresh token
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthTokens, AuthError>;

    /// Revoke refresh token (logout)
    async fn revoke_token(&self, refresh_token: &str) -> Result<(), AuthError>;

    /// Validate access token and extract user ID
    async fn validate_token(&self, access_token: &str) -> Result<i64, AuthError>;

    /// Get current user from access token
    async fn get_current_user(&self, access_token: &str) -> Result<User, AuthError>;
}

/// Authentication tokens response
#[derive(Debug, Clone, Serialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at time (Unix timestamp)
    pub iat: i64,
    /// JWT ID for token revocation tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("User not found")]
    UserNotFound,

    #[error("Email already exists")]
    EmailExists,

    #[error("Username already exists")]
    UsernameExists,

    #[error("Session not found or expired")]
    SessionNotFound,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// AuthService implementation
pub struct AuthServiceImpl<U, S>
where
    U: UserRepository,
    S: SessionRepository,
{
    user_repo: Arc<U>,
    session_repo: Arc<S>,
    id_generator: Arc<SnowflakeGenerator>,
    jwt_settings: JwtSettings,
}

impl<U, S> AuthServiceImpl<U, S>
where
    U: UserRepository,
    S: SessionRepository,
{
    /// Create a new AuthServiceImpl
    pub fn new(
        user_repo: Arc<U>,
        session_repo: Arc<S>,
        id_generator: Arc<SnowflakeGenerator>,
        jwt_settings: JwtSettings,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            id_generator,
            jwt_settings,
        }
    }

    /// Hash a password using Argon2id
    fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::Internal(format!("Password hashing failed: {}", e)))
    }

    /// Verify a password against its hash
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::Internal(format!("Invalid password hash: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate access and refresh tokens
    fn generate_tokens(&self, user_id: i64) -> Result<AuthTokens, AuthError> {
        let now = Utc::now();
        let access_expiry = now + Duration::minutes(self.jwt_settings.access_token_expiry_minutes);

        // Generate unique JWT ID for token revocation tracking
        let jti = uuid::Uuid::new_v4().to_string();

        // Generate access token with jti claim
        let access_claims = Claims {
            sub: user_id.to_string(),
            exp: access_expiry.timestamp(),
            iat: now.timestamp(),
            jti: Some(jti),
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.jwt_settings.secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(format!("Token generation failed: {}", e)))?;

        // Generate opaque refresh token (no sensitive data exposed)
        // Format: random_uuid.random_uuid (completely opaque, no user info)
        let refresh_token = format!(
            "{}.{}",
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4()
        );

        Ok(AuthTokens {
            access_token,
            refresh_token,
            expires_in: self.jwt_settings.access_token_expiry_minutes * 60,
            token_type: "Bearer".to_string(),
        })
    }

    /// Hash refresh token for storage
    fn hash_refresh_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Decode and validate access token
    fn decode_access_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_settings.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        })?;

        Ok(token_data.claims)
    }
}

#[async_trait]
impl<U, S> AuthService for AuthServiceImpl<U, S>
where
    U: UserRepository + 'static,
    S: SessionRepository + 'static,
{
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(User, AuthTokens), AuthError> {
        // Check if email already exists
        if self
            .user_repo
            .email_exists(email)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
        {
            return Err(AuthError::EmailExists);
        }

        // Check if username already exists
        if self
            .user_repo
            .username_exists(username)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
        {
            return Err(AuthError::UsernameExists);
        }

        // Hash password
        let password_hash = self.hash_password(password)?;

        // Generate user ID
        let user_id = self.id_generator.generate();

        // Create user
        let now = Utc::now();
        let user = User {
            id: user_id,
            username: username.to_string(),
            email: email.to_string(),
            password_hash,
            display_name: None,
            avatar_url: None,
            status: crate::domain::UserStatus::Online,
            bio: None,
            created_at: now,
            updated_at: now,
        };

        let created_user = self
            .user_repo
            .create(&user)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        // Generate tokens
        let tokens = self.generate_tokens(created_user.id)?;

        // Create session for refresh token
        let token_hash = self.hash_refresh_token(&tokens.refresh_token);
        let session = Session::new(
            created_user.id,
            token_hash,
            Utc::now() + Duration::days(self.jwt_settings.refresh_token_expiry_days),
        );

        self.session_repo
            .create(&session)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok((created_user, tokens))
    }

    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthTokens, AuthError> {
        // Find user by email
        let user = self
            .user_repo
            .find_by_email(email)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::InvalidCredentials)?;

        // Verify password
        if !self.verify_password(password, &user.password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        // Generate tokens
        let tokens = self.generate_tokens(user.id)?;

        // Create session
        let token_hash = self.hash_refresh_token(&tokens.refresh_token);
        let session = Session::new(
            user.id,
            token_hash,
            Utc::now() + Duration::days(self.jwt_settings.refresh_token_expiry_days),
        );

        self.session_repo
            .create(&session)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(tokens)
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthTokens, AuthError> {
        let token_hash = self.hash_refresh_token(refresh_token);

        // Find session by token hash
        let session = self
            .session_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::SessionNotFound)?;

        // Check if session is still valid
        if !session.is_active() {
            return Err(AuthError::TokenExpired);
        }

        // Generate new tokens (TOKEN ROTATION for security)
        let new_tokens = self.generate_tokens(session.user_id)?;
        let new_token_hash = self.hash_refresh_token(&new_tokens.refresh_token);
        let new_expires_at = Utc::now() + Duration::days(self.jwt_settings.refresh_token_expiry_days);

        // Update session with new refresh token hash (token rotation)
        self.session_repo
            .update_token_hash(session.id, &new_token_hash, new_expires_at)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(new_tokens)
    }

    async fn revoke_token(&self, refresh_token: &str) -> Result<(), AuthError> {
        let token_hash = self.hash_refresh_token(refresh_token);

        // Find and revoke session
        let session = self
            .session_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::SessionNotFound)?;

        self.session_repo
            .revoke(session.id)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn validate_token(&self, access_token: &str) -> Result<i64, AuthError> {
        let claims = self.decode_access_token(access_token)?;

        claims
            .sub
            .parse::<i64>()
            .map_err(|_| AuthError::InvalidToken)
    }

    async fn get_current_user(&self, access_token: &str) -> Result<User, AuthError> {
        let user_id = self.validate_token(access_token).await?;

        self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::UserNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        // Create a minimal test - actual integration tests would need mocks
    }
}
