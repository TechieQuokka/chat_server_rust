# Architecture Patterns & Implementation Guide

> **Language**: Rust 2024 Edition
> **Framework**: Axum 0.7+
> **Last Updated**: 2025-12-17

---

## 목차 (Table of Contents)

- [1. 아키텍처 개요](#1-아키텍처-개요-architecture-overview)
- [2. 프로젝트 구조](#2-프로젝트-구조-project-structure)
- [3. Repository 패턴](#3-repository-패턴)
- [4. Service Layer 패턴](#4-service-layer-패턴)
- [5. 의존성 주입](#5-의존성-주입-dependency-injection)
- [6. 에러 핸들링](#6-에러-핸들링-error-handling)
- [7. Request/Response 패턴](#7-requestresponse-패턴)
- [8. Middleware 패턴](#8-middleware-패턴)
- [9. 로깅 및 추적](#9-로깅-및-추적-logging--tracing)
- [10. 설정 관리](#10-설정-관리-configuration)
- [11. 테스트 전략](#11-테스트-전략-testing)
- [12. 비동기 패턴](#12-비동기-패턴-async-patterns)
- [13. 안티패턴 경고](#13-안티패턴-경고-anti-patterns)

---

## 1. 아키텍처 개요 (Architecture Overview)

### 1.1 Clean Architecture Layers

```mermaid
graph TB
    subgraph Presentation["Presentation Layer"]
        HTTP[HTTP Handlers]
        WS[WebSocket Handlers]
        MW[Middleware]
    end

    subgraph Application["Application Layer"]
        SVC[Services]
        DTO[DTOs]
        CMD[Commands/Queries]
    end

    subgraph Domain["Domain Layer"]
        ENT[Entities]
        REPO[Repository Traits]
        DOM_SVC[Domain Services]
    end

    subgraph Infrastructure["Infrastructure Layer"]
        PG_REPO[PostgreSQL Repository]
        REDIS[Redis Client]
        S3[S3 Client]
    end

    HTTP --> MW --> SVC
    WS --> MW --> SVC
    SVC --> REPO
    SVC --> DOM_SVC
    REPO -.->|implements| PG_REPO
    SVC --> REDIS
    SVC --> S3
```

### 1.2 Dependency Direction

- **Presentation → Application**: HTTP handlers call services
- **Application → Domain**: Services use domain entities and repository traits
- **Infrastructure → Domain**: Implementations depend on domain traits
- **Domain**: No external dependencies (pure business logic)

---

## 2. 프로젝트 구조 (Project Structure)

```
chat_server/
├── Cargo.toml
├── .env.example
├── migrations/
│   ├── 20250101000000_create_extensions.sql
│   ├── 20250101000001_create_users.sql
│   └── ...
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library exports
│   │
│   ├── config/                 # Configuration
│   │   ├── mod.rs
│   │   └── settings.rs
│   │
│   ├── domain/                 # Domain layer
│   │   ├── mod.rs
│   │   ├── entities/           # Domain entities
│   │   │   ├── mod.rs
│   │   │   ├── user.rs
│   │   │   ├── server.rs
│   │   │   ├── channel.rs
│   │   │   └── message.rs
│   │   └── repositories/       # Repository traits
│   │       ├── mod.rs
│   │       ├── user_repository.rs
│   │       └── message_repository.rs
│   │
│   ├── application/            # Application layer
│   │   ├── mod.rs
│   │   ├── services/           # Application services
│   │   │   ├── mod.rs
│   │   │   ├── auth_service.rs
│   │   │   ├── user_service.rs
│   │   │   └── message_service.rs
│   │   └── dto/                # Data Transfer Objects
│   │       ├── mod.rs
│   │       ├── requests.rs
│   │       └── responses.rs
│   │
│   ├── infrastructure/         # Infrastructure layer
│   │   ├── mod.rs
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── pg_user_repository.rs
│   │   │   └── pg_message_repository.rs
│   │   ├── cache/
│   │   │   ├── mod.rs
│   │   │   └── redis_client.rs
│   │   └── storage/
│   │       └── s3_client.rs
│   │
│   ├── api/                    # Presentation layer
│   │   ├── mod.rs
│   │   ├── routes/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs
│   │   │   ├── users.rs
│   │   │   ├── guilds.rs
│   │   │   ├── channels.rs
│   │   │   └── messages.rs
│   │   ├── handlers/
│   │   │   └── mod.rs
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs
│   │   │   └── rate_limit.rs
│   │   └── extractors/
│   │       ├── mod.rs
│   │       └── auth.rs
│   │
│   ├── ws/                     # WebSocket gateway
│   │   ├── mod.rs
│   │   ├── gateway.rs
│   │   ├── session.rs
│   │   ├── events.rs
│   │   └── opcodes.rs
│   │
│   ├── shared/                 # Shared utilities
│   │   ├── mod.rs
│   │   ├── error.rs
│   │   ├── snowflake.rs
│   │   └── permissions.rs
│   │
│   └── state.rs                # Application state
│
└── tests/
    ├── integration/
    │   ├── auth_tests.rs
    │   └── message_tests.rs
    └── fixtures/
        └── mod.rs
```

---

## 3. Repository 패턴

### 3.1 Repository Trait Definition

```rust
// src/domain/repositories/user_repository.rs
use async_trait::async_trait;
use crate::domain::entities::User;
use crate::shared::error::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find user by internal ID
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError>;

    /// Find user by email (for login)
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;

    /// Find user by username and discriminator
    async fn find_by_username(
        &self,
        username: &str,
        discriminator: &str,
    ) -> Result<Option<User>, AppError>;

    /// Create a new user
    async fn create(&self, user: &User) -> Result<User, AppError>;

    /// Update user
    async fn update(&self, user: &User) -> Result<User, AppError>;

    /// Soft delete user
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Check if email exists
    async fn email_exists(&self, email: &str) -> Result<bool, AppError>;
}
```

### 3.2 PostgreSQL Implementation

```rust
// src/infrastructure/database/pg_user_repository.rs
use async_trait::async_trait;
use sqlx::PgPool;
use crate::domain::entities::User;
use crate::domain::repositories::UserRepository;
use crate::shared::error::AppError;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id, public_id, username, discriminator, email,
                password_hash, avatar_url, banner_url, bio,
                is_verified, is_bot, premium_type,
                created_at, updated_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id, public_id, username, discriminator, email,
                password_hash, avatar_url, banner_url, bio,
                is_verified, is_bot, premium_type,
                created_at, updated_at
            FROM users
            WHERE lower(email) = lower($1) AND deleted_at IS NULL
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn create(&self, user: &User) -> Result<User, AppError> {
        let created = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                username, discriminator, email, password_hash,
                avatar_url, bio, is_verified
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, public_id, username, discriminator, email,
                password_hash, avatar_url, banner_url, bio,
                is_verified, is_bot, premium_type,
                created_at, updated_at
            "#,
            user.username,
            user.discriminator,
            user.email,
            user.password_hash,
            user.avatar_url,
            user.bio,
            user.is_verified
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(created)
    }

    async fn update(&self, user: &User) -> Result<User, AppError> {
        let updated = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET
                username = COALESCE($2, username),
                avatar_url = $3,
                banner_url = $4,
                bio = $5,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id, public_id, username, discriminator, email,
                password_hash, avatar_url, banner_url, bio,
                is_verified, is_bot, premium_type,
                created_at, updated_at
            "#,
            user.id,
            user.username,
            user.avatar_url,
            user.banner_url,
            user.bio
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE users SET deleted_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn email_exists(&self, email: &str) -> Result<bool, AppError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users
                WHERE lower(email) = lower($1) AND deleted_at IS NULL
            ) as "exists!"
            "#,
            email
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_username(
        &self,
        username: &str,
        discriminator: &str,
    ) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id, public_id, username, discriminator, email,
                password_hash, avatar_url, banner_url, bio,
                is_verified, is_bot, premium_type,
                created_at, updated_at
            FROM users
            WHERE username = $1 AND discriminator = $2 AND deleted_at IS NULL
            "#,
            username,
            discriminator
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
}
```

### 3.3 Transaction Support

```rust
// src/infrastructure/database/mod.rs
use sqlx::{PgPool, Transaction, Postgres};
use crate::shared::error::AppError;

pub struct UnitOfWork {
    tx: Transaction<'static, Postgres>,
}

impl UnitOfWork {
    pub async fn begin(pool: &PgPool) -> Result<Self, AppError> {
        let tx = pool.begin().await?;
        Ok(Self { tx })
    }

    pub async fn commit(self) -> Result<(), AppError> {
        self.tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<(), AppError> {
        self.tx.rollback().await?;
        Ok(())
    }

    pub fn transaction(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.tx
    }
}

// Usage in service
pub async fn transfer_ownership(
    pool: &PgPool,
    server_id: i64,
    new_owner_id: i64,
) -> Result<(), AppError> {
    let mut uow = UnitOfWork::begin(pool).await?;

    // Multiple operations in single transaction
    sqlx::query!(
        "UPDATE servers SET owner_id = $1 WHERE id = $2",
        new_owner_id, server_id
    )
    .execute(uow.transaction())
    .await?;

    sqlx::query!(
        "INSERT INTO audit_logs (server_id, action_type, target_id) VALUES ($1, $2, $3)",
        server_id, 1, new_owner_id
    )
    .execute(uow.transaction())
    .await?;

    uow.commit().await?;
    Ok(())
}
```

---

## 4. Service Layer 패턴

### 4.1 Service Trait

```rust
// src/application/services/auth_service.rs
use async_trait::async_trait;
use crate::application::dto::{
    LoginRequest, RegisterRequest, AuthResponse, TokenPair
};
use crate::shared::error::AppError;

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AppError>;
    async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AppError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, AppError>;
    async fn logout(&self, refresh_token: &str) -> Result<(), AppError>;
    async fn validate_token(&self, token: &str) -> Result<i64, AppError>;
}
```

### 4.2 Service Implementation

```rust
// src/application/services/auth_service_impl.rs
use std::sync::Arc;
use async_trait::async_trait;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};

use crate::domain::entities::User;
use crate::domain::repositories::UserRepository;
use crate::application::dto::*;
use crate::shared::error::AppError;
use crate::config::Settings;

pub struct AuthServiceImpl {
    user_repo: Arc<dyn UserRepository>,
    config: Arc<Settings>,
    redis: Arc<redis::Client>,
}

impl AuthServiceImpl {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        config: Arc<Settings>,
        redis: Arc<redis::Client>,
    ) -> Self {
        Self { user_repo, config, redis }
    }

    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn generate_tokens(&self, user_id: i64, session_id: &str) -> Result<TokenPair, AppError> {
        let now = chrono::Utc::now();

        // Access token (15 minutes)
        let access_claims = Claims {
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            exp: (now + chrono::Duration::minutes(15)).timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "access".to_string(),
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )?;

        // Refresh token (7 days)
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            exp: (now + chrono::Duration::days(7)).timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "refresh".to_string(),
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: 900, // 15 minutes
        })
    }

    async fn generate_discriminator(&self, username: &str) -> Result<String, AppError> {
        // Try random discriminators until we find an unused one
        for _ in 0..10 {
            let disc = format!("{:04}", rand::random::<u16>() % 10000);
            if self.user_repo.find_by_username(username, &disc).await?.is_none() {
                return Ok(disc);
            }
        }
        Err(AppError::Conflict("No available discriminator".to_string()))
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AppError> {
        // Validate email uniqueness
        if self.user_repo.email_exists(&request.email).await? {
            return Err(AppError::Conflict("Email already registered".to_string()));
        }

        // Generate password hash
        let password_hash = self.hash_password(&request.password)?;

        // Generate unique discriminator
        let discriminator = self.generate_discriminator(&request.username).await?;

        // Create user
        let user = User {
            id: 0, // Will be generated
            public_id: uuid::Uuid::new_v4(),
            username: request.username,
            discriminator,
            email: request.email,
            password_hash,
            avatar_url: None,
            banner_url: None,
            bio: None,
            is_verified: false,
            is_bot: false,
            premium_type: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created_user = self.user_repo.create(&user).await?;

        // Generate session and tokens
        let session_id = uuid::Uuid::new_v4().to_string();
        let tokens = self.generate_tokens(created_user.id, &session_id)?;

        Ok(AuthResponse {
            user: created_user.into(),
            tokens,
        })
    }

    async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AppError> {
        // Find user by email
        let user = self.user_repo
            .find_by_email(&request.email)
            .await?
            .ok_or(AppError::Unauthorized("Invalid credentials".to_string()))?;

        // Verify password
        if !self.verify_password(&request.password, &user.password_hash)? {
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        // Generate session and tokens
        let session_id = uuid::Uuid::new_v4().to_string();
        let tokens = self.generate_tokens(user.id, &session_id)?;

        Ok(AuthResponse {
            user: user.into(),
            tokens,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, AppError> {
        // Decode and validate refresh token
        let token_data = decode::<Claims>(
            refresh_token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::default(),
        ).map_err(|_| AppError::Unauthorized("Invalid refresh token".to_string()))?;

        if token_data.claims.token_type != "refresh" {
            return Err(AppError::Unauthorized("Invalid token type".to_string()));
        }

        let user_id: i64 = token_data.claims.sub.parse()
            .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

        // Generate new tokens
        let new_session_id = uuid::Uuid::new_v4().to_string();
        self.generate_tokens(user_id, &new_session_id)
    }

    async fn logout(&self, refresh_token: &str) -> Result<(), AppError> {
        // Add token to blacklist in Redis
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        redis::cmd("SETEX")
            .arg(format!("blacklist:{}", refresh_token))
            .arg(604800) // 7 days
            .arg("1")
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn validate_token(&self, token: &str) -> Result<i64, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::default(),
        ).map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

        let user_id: i64 = token_data.claims.sub.parse()
            .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

        Ok(user_id)
    }
}
```

---

## 5. 의존성 주입 (Dependency Injection)

### 5.1 AppState Pattern

```rust
// src/state.rs
use std::sync::Arc;
use sqlx::PgPool;
use redis::Client as RedisClient;

use crate::config::Settings;
use crate::domain::repositories::*;
use crate::application::services::*;
use crate::infrastructure::database::*;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Settings>,
    pub db_pool: PgPool,
    pub redis: Arc<RedisClient>,

    // Repositories
    pub user_repo: Arc<dyn UserRepository>,
    pub message_repo: Arc<dyn MessageRepository>,

    // Services
    pub auth_service: Arc<dyn AuthService>,
    pub user_service: Arc<dyn UserService>,
    pub message_service: Arc<dyn MessageService>,
}

impl AppState {
    pub async fn new(config: Settings) -> Result<Self, anyhow::Error> {
        let config = Arc::new(config);

        // Database connection pool
        let db_pool = PgPool::connect(&config.database_url).await?;

        // Redis client
        let redis = Arc::new(RedisClient::open(config.redis_url.clone())?);

        // Initialize repositories
        let user_repo: Arc<dyn UserRepository> =
            Arc::new(PgUserRepository::new(db_pool.clone()));
        let message_repo: Arc<dyn MessageRepository> =
            Arc::new(PgMessageRepository::new(db_pool.clone()));

        // Initialize services
        let auth_service: Arc<dyn AuthService> = Arc::new(
            AuthServiceImpl::new(
                user_repo.clone(),
                config.clone(),
                redis.clone(),
            )
        );

        let user_service: Arc<dyn UserService> = Arc::new(
            UserServiceImpl::new(user_repo.clone())
        );

        let message_service: Arc<dyn MessageService> = Arc::new(
            MessageServiceImpl::new(
                message_repo.clone(),
                redis.clone(),
            )
        );

        Ok(Self {
            config,
            db_pool,
            redis,
            user_repo,
            message_repo,
            auth_service,
            user_service,
            message_service,
        })
    }
}
```

### 5.2 Service Registration in main.rs

```rust
// src/main.rs
use axum::{Router, Extension};
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod config;
mod state;
mod api;
mod domain;
mod application;
mod infrastructure;
mod shared;
mod ws;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = config::Settings::load()?;
    let addr = config.server_addr();

    // Create application state
    let state = state::AppState::new(config).await?;

    // Build router
    let app = Router::new()
        .merge(api::routes::auth::router())
        .merge(api::routes::users::router())
        .merge(api::routes::guilds::router())
        .merge(api::routes::channels::router())
        .merge(api::routes::messages::router())
        .merge(ws::gateway::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    tracing::info!("Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## 6. 에러 핸들링 (Error Handling)

### 6.1 Error Type Hierarchy

```rust
// src/shared/error.rs
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // Authentication errors
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    // Resource errors
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    // Validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    // Rate limiting
    #[error("Rate limited")]
    RateLimited { retry_after: u64 },

    // Internal errors
    #[error("Internal server error: {0}")]
    InternalError(String),

    // Database errors
    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),

    // Redis errors
    #[error("Cache error")]
    CacheError(#[from] redis::RedisError),

    // JWT errors
    #[error("Token error")]
    TokenError(#[from] jsonwebtoken::errors::Error),
}

impl AppError {
    pub fn error_code(&self) -> u32 {
        match self {
            Self::Unauthorized(_) => 40001,
            Self::Forbidden(_) => 50001,
            Self::NotFound(_) => 10000,
            Self::Conflict(_) => 40009,
            Self::ValidationError(_) => 50035,
            Self::BadRequest(_) => 50035,
            Self::RateLimited { .. } => 42900,
            Self::InternalError(_) => 50000,
            Self::DatabaseError(_) => 50000,
            Self::CacheError(_) => 50000,
            Self::TokenError(_) => 50014,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::CacheError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TokenError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();

        // Don't expose internal details to clients
        let message = match &self {
            Self::DatabaseError(_) | Self::CacheError(_) | Self::InternalError(_) => {
                tracing::error!("Internal error: {:?}", self);
                "Internal server error".to_string()
            }
            _ => self.to_string(),
        };

        let body = json!({
            "error": {
                "code": error_code,
                "message": message
            }
        });

        (status, Json(body)).into_response()
    }
}

// Convenience type alias
pub type AppResult<T> = Result<T, AppError>;
```

### 6.2 Error Propagation in Handlers

```rust
// src/api/routes/users.rs
use axum::{
    extract::{Path, State},
    Json,
};
use crate::state::AppState;
use crate::shared::error::{AppError, AppResult};
use crate::api::extractors::AuthUser;
use crate::application::dto::UserResponse;

pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    _auth: AuthUser, // Ensures authentication
) -> AppResult<Json<UserResponse>> {
    let user = state.user_service
        .get_user(user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

    Ok(Json(user.into()))
}
```

---

## 7. Request/Response 패턴

### 7.1 Request DTOs with Validation

```rust
// src/application/dto/requests.rs
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 2, max = 32))]
    #[validate(regex(path = "USERNAME_REGEX"))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 128))]
    #[validate(custom = "validate_password")]
    pub password: String,

    pub date_of_birth: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateMessageRequest {
    #[validate(length(max = 2000))]
    pub content: Option<String>,

    pub tts: Option<bool>,

    #[validate(length(max = 10))]
    pub embeds: Option<Vec<EmbedRequest>>,

    pub message_reference: Option<MessageReferenceRequest>,
}

// Custom password validator
fn validate_password(password: &str) -> Result<(), validator::ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if has_uppercase && has_lowercase && has_digit {
        Ok(())
    } else {
        Err(validator::ValidationError::new("weak_password"))
    }
}

lazy_static::lazy_static! {
    static ref USERNAME_REGEX: regex::Regex =
        regex::Regex::new(r"^[a-zA-Z0-9_]{2,32}$").unwrap();
}
```

### 7.2 Response DTOs

```rust
// src/application/dto/responses.rs
use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    pub verified: bool,
    pub premium_type: i16,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            discriminator: user.discriminator,
            avatar: user.avatar_url,
            banner: user.banner_url,
            bio: user.bio,
            verified: user.is_verified,
            premium_type: user.premium_type,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub author: UserResponse,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub edited_timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "type")]
    pub message_type: i16,
    pub flags: i32,
}
```

### 7.3 Complete Handler Pattern

```rust
// src/api/routes/messages.rs
use axum::{
    extract::{Path, Query, State},
    Json,
    routing::{get, post},
    Router,
};
use validator::Validate;

use crate::state::AppState;
use crate::shared::error::{AppError, AppResult};
use crate::api::extractors::AuthUser;
use crate::application::dto::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/:channel_id/messages", get(get_messages).post(create_message))
        .route("/channels/:channel_id/messages/:message_id", get(get_message).patch(edit_message).delete(delete_message))
}

#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    before: Option<i64>,
    after: Option<i64>,
    around: Option<i64>,
    limit: Option<i32>,
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
    Query(query): Query<GetMessagesQuery>,
    auth: AuthUser,
) -> AppResult<Json<Vec<MessageResponse>>> {
    // Check permission
    state.permission_service
        .check_channel_permission(auth.user_id, channel_id, Permission::VIEW_CHANNEL)
        .await?;

    let limit = query.limit.unwrap_or(50).min(100);

    let messages = state.message_service
        .get_messages(channel_id, query.before, query.after, limit)
        .await?;

    Ok(Json(messages.into_iter().map(Into::into).collect()))
}

pub async fn create_message(
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
    auth: AuthUser,
    Json(request): Json<CreateMessageRequest>,
) -> AppResult<Json<MessageResponse>> {
    // Validate request
    request.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Check permission
    state.permission_service
        .check_channel_permission(auth.user_id, channel_id, Permission::SEND_MESSAGES)
        .await?;

    // Must have content or embeds
    if request.content.is_none() && request.embeds.as_ref().map_or(true, |e| e.is_empty()) {
        return Err(AppError::BadRequest("Message must have content or embeds".to_string()));
    }

    let message = state.message_service
        .create_message(auth.user_id, channel_id, request)
        .await?;

    Ok(Json(message.into()))
}
```

---

## 8. Middleware 패턴

### 8.1 Authentication Middleware

```rust
// src/api/middleware/auth.rs
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::header::AUTHORIZATION,
};
use crate::state::AppState;
use crate::shared::error::AppError;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized("Missing authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Invalid authorization format".to_string()));
    }

    let token = &auth_header[7..];
    let user_id = state.auth_service.validate_token(token).await?;

    // Add user_id to request extensions
    request.extensions_mut().insert(AuthenticatedUser { user_id });

    Ok(next.run(request).await)
}

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: i64,
}
```

### 8.2 Auth Extractor

```rust
// src/api/extractors/auth.rs
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use crate::shared::error::AppError;
use crate::api::middleware::auth::AuthenticatedUser;

pub struct AuthUser {
    pub user_id: i64,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or(AppError::Unauthorized("Not authenticated".to_string()))?;

        Ok(AuthUser {
            user_id: auth.user_id,
        })
    }
}
```

### 8.3 Rate Limiting Middleware

```rust
// src/api/middleware/rate_limit.rs
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use crate::shared::error::AppError;

pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
    config: RateLimitConfig,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub tokens_per_second: f64,
    pub bucket_size: f64,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: DashMap::new(),
            config,
        }
    }

    pub fn check(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();

        let mut bucket = self.buckets.entry(key.to_string()).or_insert(TokenBucket {
            tokens: self.config.bucket_size,
            last_update: now,
        });

        // Refill tokens
        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.config.tokens_per_second)
            .min(self.config.bucket_size);
        bucket.last_update = now;

        // Try to consume a token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate retry after
            let needed = 1.0 - bucket.tokens;
            let retry_after = (needed / self.config.tokens_per_second).ceil() as u64;
            Err(retry_after)
        }
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Get rate limit key (IP or user ID)
    let key = request
        .extensions()
        .get::<AuthenticatedUser>()
        .map(|u| format!("user:{}", u.user_id))
        .unwrap_or_else(|| {
            request
                .headers()
                .get("X-Forwarded-For")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
                .to_string()
        });

    limiter.check(&key).map_err(|retry_after| {
        AppError::RateLimited { retry_after }
    })?;

    Ok(next.run(request).await)
}
```

---

## 9. 로깅 및 추적 (Logging & Tracing)

### 9.1 Tracing Setup

```rust
// src/main.rs
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug,sqlx=warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
```

### 9.2 Request Tracing

```rust
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;

let app = Router::new()
    // ... routes
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO))
    );
```

---

## 10. 설정 관리 (Configuration)

### 10.1 Config Structure

```rust
// src/config/settings.rs
use serde::Deserialize;
use config::{Config, Environment, File};

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_minutes: i64,
    pub environment: String,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        let env = std::env::var("RUN_ENV").unwrap_or_else(|_| "development".into());

        Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize()
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
```

### 10.2 Environment Variables

```bash
# .env.example
APP__SERVER_HOST=0.0.0.0
APP__SERVER_PORT=3000
APP__DATABASE_URL=postgres://user:pass@localhost/chat_server
APP__REDIS_URL=redis://localhost:6379
APP__JWT_SECRET=your-super-secret-key-here-change-in-production
APP__JWT_EXPIRY_MINUTES=15
APP__ENVIRONMENT=development
```

---

## 11. 테스트 전략 (Testing)

### 11.1 Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_register_success() {
        let mut mock_repo = MockUserRepository::new();

        mock_repo
            .expect_email_exists()
            .with(eq("test@example.com"))
            .returning(|_| Ok(false));

        mock_repo
            .expect_find_by_username()
            .returning(|_, _| Ok(None));

        mock_repo
            .expect_create()
            .returning(|u| Ok(u.clone()));

        let service = AuthServiceImpl::new(
            Arc::new(mock_repo),
            Arc::new(test_config()),
            Arc::new(test_redis()),
        );

        let request = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
            date_of_birth: None,
        };

        let result = service.register(request).await;
        assert!(result.is_ok());
    }
}
```

### 11.2 Integration Test

```rust
// tests/integration/auth_tests.rs
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn test_register_and_login() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Register
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    // Login
    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["tokens"]["access_token"].is_string());
}
```

---

## 12. 비동기 패턴 (Async Patterns)

### 12.1 Graceful Shutdown

```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

// In main
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

---

## 13. 안티패턴 경고 (Anti-patterns)

### ❌ Fat Repository

```rust
// BAD: Business logic in repository
impl UserRepository for PgUserRepository {
    async fn create(&self, user: &User) -> Result<User, AppError> {
        // DON'T: Password hashing in repository
        let hash = hash_password(&user.password)?;
        // ...
    }
}
```

### ✅ Correct: Business Logic in Service

```rust
// GOOD: Service handles business logic
impl AuthService for AuthServiceImpl {
    async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AppError> {
        let hash = self.hash_password(&request.password)?;
        let user = User { password_hash: hash, ..Default::default() };
        self.user_repo.create(&user).await
    }
}
```

### ❌ Blocking in Async Context

```rust
// BAD: Blocking call in async function
async fn hash_password(password: &str) -> Result<String, AppError> {
    // This blocks the entire async runtime!
    let hash = argon2::hash_encoded(password.as_bytes(), &salt, &config)?;
    Ok(hash)
}
```

### ✅ Correct: Use spawn_blocking

```rust
// GOOD: Use spawn_blocking for CPU-intensive work
async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2.hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await?
    .map_err(|e| AppError::InternalError(e.to_string()))
}
```

---

*This document provides complete implementation patterns for the Discord-like chat server.*
