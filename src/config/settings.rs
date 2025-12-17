//! Application settings and configuration structures.

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// Root configuration structure containing all application settings.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Server configuration (host, port)
    pub server: ServerSettings,

    /// Database configuration (PostgreSQL)
    pub database: DatabaseSettings,

    /// Redis configuration
    pub redis: RedisSettings,

    /// JWT authentication settings
    pub jwt: JwtSettings,

    /// Snowflake ID generator settings
    pub snowflake: SnowflakeSettings,

    /// Rate limiting configuration
    pub rate_limit: RateLimitSettings,

    /// CORS configuration
    pub cors: CorsSettings,

    /// Current environment (development, staging, production)
    pub environment: String,
}

/// Server binding configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    /// Host address to bind to (e.g., "0.0.0.0")
    pub host: String,

    /// Port number to listen on
    pub port: u16,
}

/// PostgreSQL database configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    /// Database connection URL
    pub url: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    pub min_connections: u32,

    /// Connection acquire timeout in seconds
    pub acquire_timeout: u64,
}

/// Redis configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RedisSettings {
    /// Redis connection URL
    pub url: String,

    /// Connection pool size
    pub pool_size: u32,
}

/// JWT authentication configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtSettings {
    /// Secret key for signing tokens
    pub secret: String,

    /// Access token expiry in minutes
    pub access_token_expiry_minutes: i64,

    /// Refresh token expiry in days
    pub refresh_token_expiry_days: i64,
}

/// Snowflake ID generator configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SnowflakeSettings {
    /// Machine/worker ID (0-1023)
    pub machine_id: u16,

    /// Custom epoch timestamp in milliseconds
    pub epoch: u64,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSettings {
    /// Maximum requests per second
    pub requests_per_second: f64,

    /// Burst size (bucket capacity)
    pub burst_size: u32,
}

/// CORS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CorsSettings {
    /// Allowed origins (comma-separated in env)
    pub allowed_origins: Vec<String>,
}

impl Settings {
    /// Load settings from environment variables and configuration files.
    ///
    /// The loading order is:
    /// 1. config/default.toml (base configuration)
    /// 2. config/{RUN_ENV}.toml (environment-specific overrides)
    /// 3. Environment variables (highest priority)
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if configuration cannot be loaded or parsed.
    pub fn load() -> Result<Self, ConfigError> {
        // Load .env file if present (ignore errors if not found)
        let _ = dotenvy::dotenv();

        // Determine the running environment
        let environment = std::env::var("RUN_ENV").unwrap_or_else(|_| "development".into());

        Config::builder()
            // Start with default values
            .set_default("environment", environment.clone())?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 2)?
            .set_default("database.acquire_timeout", 30)?
            .set_default("redis.pool_size", 10)?
            .set_default("jwt.access_token_expiry_minutes", 15)?
            .set_default("jwt.refresh_token_expiry_days", 7)?
            .set_default("snowflake.machine_id", 1)?
            .set_default("snowflake.epoch", 1420070400000_u64)?
            .set_default("rate_limit.requests_per_second", 10.0)?
            .set_default("rate_limit.burst_size", 30)?
            .set_default("cors.allowed_origins", vec!["http://localhost:3000"])?
            // Load from config files
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", environment)).required(false))
            // Load from environment variables
            // APP__SERVER__PORT=3000 -> server.port = 3000
            .add_source(
                Environment::default()
                    .prefix("APP")
                    .separator("__")
                    .try_parsing(true),
            )
            // Map simple environment variables
            .set_override_option(
                "server.host",
                std::env::var("SERVER_HOST").ok(),
            )?
            .set_override_option(
                "server.port",
                std::env::var("SERVER_PORT").ok(),
            )?
            .set_override_option(
                "database.url",
                std::env::var("DATABASE_URL").ok(),
            )?
            .set_override_option(
                "redis.url",
                std::env::var("REDIS_URL").ok(),
            )?
            .set_override_option(
                "jwt.secret",
                std::env::var("JWT_SECRET").ok(),
            )?
            .set_override_option(
                "snowflake.machine_id",
                std::env::var("SNOWFLAKE_MACHINE_ID").ok(),
            )?
            .build()?
            .try_deserialize()
    }

    /// Get the full server address as a string.
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl ServerSettings {
    /// Get the socket address for binding.
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid server address configuration")
    }
}

impl DatabaseSettings {
    /// Get the connection URL.
    pub fn connection_url(&self) -> &str {
        &self.url
    }
}
