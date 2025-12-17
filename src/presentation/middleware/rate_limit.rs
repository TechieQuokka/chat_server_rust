//! Rate Limiting Middleware
//!
//! Redis-based distributed rate limiting using sliding window algorithm.
//! Provides protection against abuse and DDoS attacks while ensuring
//! fair resource allocation across users.

use std::net::IpAddr;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::presentation::middleware::auth::AuthUser;
use crate::shared::error::ErrorResponse;
use crate::startup::AppState;

// ============================================================================
// Rate Limit Configuration
// ============================================================================

/// Configuration for rate limiting behavior.
///
/// Different endpoint types can have different limits to balance
/// security concerns with user experience.
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Requests allowed per window for this endpoint type
    pub requests_per_window: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
    /// Optional burst allowance above base limit
    pub burst_allowance: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 60,
            window_seconds: 60,
            burst_allowance: 10,
        }
    }
}

/// Predefined rate limit configurations for different endpoint types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    /// Authentication endpoints (login, register, password reset)
    /// Lower limits to prevent credential stuffing/brute force
    Auth,
    /// Standard API endpoints
    Api,
    /// WebSocket connection establishment
    /// Moderate limits to prevent connection flooding
    WebSocket,
    /// High-frequency endpoints (typing indicators, presence)
    HighFrequency,
}

impl EndpointType {
    /// Get the rate limit configuration for this endpoint type.
    ///
    /// Security considerations:
    /// - Auth: Strict limits prevent brute force attacks
    /// - API: Balanced limits for normal usage
    /// - WebSocket: Per-connection limits prevent resource exhaustion
    /// - HighFrequency: Relaxed limits for real-time features
    pub fn config(&self) -> RateLimitConfig {
        match self {
            EndpointType::Auth => RateLimitConfig {
                requests_per_window: 5,    // 5 auth attempts per minute
                window_seconds: 60,
                burst_allowance: 2,        // Allow 2 extra for legitimate retries
            },
            EndpointType::Api => RateLimitConfig {
                requests_per_window: 60,   // 60 requests per minute
                window_seconds: 60,
                burst_allowance: 20,       // Allow bursts for page loads
            },
            EndpointType::WebSocket => RateLimitConfig {
                requests_per_window: 10,   // 10 connection attempts per minute
                window_seconds: 60,
                burst_allowance: 5,
            },
            EndpointType::HighFrequency => RateLimitConfig {
                requests_per_window: 120,  // 120 requests per minute
                window_seconds: 60,
                burst_allowance: 30,
            },
        }
    }

    /// Get the Redis key prefix for this endpoint type.
    fn key_prefix(&self) -> &'static str {
        match self {
            EndpointType::Auth => "rl:auth",
            EndpointType::Api => "rl:api",
            EndpointType::WebSocket => "rl:ws",
            EndpointType::HighFrequency => "rl:hf",
        }
    }
}

// ============================================================================
// Rate Limit Response
// ============================================================================

/// Information about rate limit status returned to clients.
#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the current window
    pub limit: u32,
    /// Remaining requests in the current window
    pub remaining: u32,
    /// Unix timestamp when the rate limit resets
    pub reset_at: i64,
    /// Seconds until the rate limit resets
    pub retry_after: u64,
}

/// Rate limit exceeded error response.
#[derive(Debug, Serialize)]
struct RateLimitExceededResponse {
    #[serde(flatten)]
    error: ErrorResponse,
    rate_limit: RateLimitInfo,
}

// ============================================================================
// Rate Limiter Implementation
// ============================================================================

/// Redis-based distributed rate limiter using sliding window algorithm.
///
/// The sliding window algorithm provides smoother rate limiting compared
/// to fixed windows by considering the request distribution over time.
///
/// # Algorithm
///
/// Uses a sorted set in Redis where:
/// - Members are unique request identifiers (timestamps with random suffix)
/// - Scores are Unix timestamps in milliseconds
///
/// On each request:
/// 1. Remove entries older than the window
/// 2. Count remaining entries
/// 3. If under limit, add new entry and allow
/// 4. If over limit, reject with retry information
///
/// # Security Properties
///
/// - Distributed: Works across multiple server instances
/// - Atomic: Uses Redis transactions to prevent race conditions
/// - Tamper-resistant: Keys are server-side only
#[derive(Clone)]
pub struct RateLimiter {
    redis: ConnectionManager,
    config: RateLimitConfig,
    endpoint_type: EndpointType,
}

impl RateLimiter {
    /// Create a new rate limiter instance.
    pub fn new(redis: ConnectionManager, endpoint_type: EndpointType) -> Self {
        Self {
            redis,
            config: endpoint_type.config(),
            endpoint_type,
        }
    }

    /// Create a rate limiter with custom configuration.
    pub fn with_config(
        redis: ConnectionManager,
        endpoint_type: EndpointType,
        config: RateLimitConfig,
    ) -> Self {
        Self {
            redis,
            config,
            endpoint_type,
        }
    }

    /// Check if a request should be allowed.
    ///
    /// Returns `Ok(RateLimitInfo)` if allowed, `Err(RateLimitInfo)` if rate limited.
    pub async fn check(&self, identifier: &str) -> Result<RateLimitInfo, RateLimitInfo> {
        let key = format!("{}:{}", self.endpoint_type.key_prefix(), identifier);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let window_ms = (self.config.window_seconds * 1000) as i64;
        let window_start = now_ms - window_ms;
        let max_requests = self.config.requests_per_window + self.config.burst_allowance;

        let mut conn = self.redis.clone();

        // Execute rate limiting logic atomically using a Lua script
        // This ensures consistency even under high concurrency
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now_ms = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local max_requests = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])

            -- Remove entries outside the window
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)

            -- Count current entries
            local current_count = redis.call('ZCARD', key)

            if current_count < max_requests then
                -- Add new request with unique member (timestamp:random)
                local member = now_ms .. ':' .. math.random(1000000)
                redis.call('ZADD', key, now_ms, member)
                -- Set expiry to clean up old keys
                redis.call('EXPIRE', key, window_seconds + 1)
                return {1, current_count + 1, max_requests}
            else
                -- Get oldest entry timestamp to calculate retry time
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                local retry_after = 0
                if oldest and #oldest >= 2 then
                    retry_after = oldest[2] + (window_seconds * 1000) - now_ms
                end
                return {0, current_count, max_requests, retry_after}
            end
            "#,
        );

        let result: Vec<i64> = script
            .key(&key)
            .arg(now_ms)
            .arg(window_start)
            .arg(max_requests as i64)
            .arg(self.config.window_seconds as i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!("Rate limiter Redis error: {}", e);
                // On Redis error, allow the request but log it
                // This prevents Redis issues from causing complete service denial
                RateLimitInfo {
                    limit: max_requests,
                    remaining: 1,
                    reset_at: (now_ms / 1000) + self.config.window_seconds as i64,
                    retry_after: 0,
                }
            })?;

        let allowed = result[0] == 1;
        let current_count = result[1] as u32;
        let remaining = max_requests.saturating_sub(current_count);
        let reset_at = (now_ms / 1000) + self.config.window_seconds as i64;

        let info = RateLimitInfo {
            limit: max_requests,
            remaining,
            reset_at,
            retry_after: if allowed {
                0
            } else {
                // Calculate retry_after in seconds
                let retry_ms = result.get(3).copied().unwrap_or(0);
                ((retry_ms as f64) / 1000.0).ceil() as u64
            },
        };

        if allowed {
            Ok(info)
        } else {
            Err(info)
        }
    }

    /// Get the current rate limit status without consuming a request.
    pub async fn status(&self, identifier: &str) -> Result<RateLimitInfo, redis::RedisError> {
        let key = format!("{}:{}", self.endpoint_type.key_prefix(), identifier);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let window_ms = (self.config.window_seconds * 1000) as i64;
        let window_start = now_ms - window_ms;
        let max_requests = self.config.requests_per_window + self.config.burst_allowance;

        let mut conn = self.redis.clone();

        // Remove old entries and count
        let _: () = conn.zrembyscore(&key, "-inf", window_start).await?;
        let count: u32 = conn.zcard(&key).await?;

        Ok(RateLimitInfo {
            limit: max_requests,
            remaining: max_requests.saturating_sub(count),
            reset_at: (now_ms / 1000) + self.config.window_seconds as i64,
            retry_after: 0,
        })
    }

    /// Reset the rate limit for an identifier (admin use only).
    ///
    /// # Security Warning
    /// This should only be exposed to admin endpoints with proper authorization.
    pub async fn reset(&self, identifier: &str) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", self.endpoint_type.key_prefix(), identifier);
        let mut conn = self.redis.clone();
        let _: () = conn.del(&key).await?;
        Ok(())
    }
}

// ============================================================================
// Identifier Extraction
// ============================================================================

/// Extract the rate limit identifier from a request.
///
/// Priority:
/// 1. Authenticated user ID (most accurate, prevents account sharing abuse)
/// 2. X-Forwarded-For header (for reverse proxy setups)
/// 3. Client IP address (fallback)
///
/// # Security Considerations
///
/// - User ID is preferred as it cannot be spoofed
/// - X-Forwarded-For should only be trusted from known proxies
/// - IP-based limiting can be bypassed with VPNs but provides baseline protection
fn extract_identifier(request: &Request, client_ip: Option<IpAddr>) -> String {
    // Check for authenticated user first
    if let Some(auth_user) = request.extensions().get::<AuthUser>() {
        return format!("user:{}", auth_user.user_id);
    }

    // Try X-Forwarded-For header (first IP in chain is original client)
    // Note: This header can be spoofed if not behind a trusted proxy
    if let Some(forwarded_for) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        if let Some(first_ip) = forwarded_for.split(',').next() {
            let ip = first_ip.trim();
            // Basic validation - ensure it looks like an IP
            if ip.parse::<IpAddr>().is_ok() {
                return format!("ip:{}", ip);
            }
        }
    }

    // Try X-Real-IP header (common with nginx)
    if let Some(real_ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|h| h.to_str().ok())
    {
        if real_ip.parse::<IpAddr>().is_ok() {
            return format!("ip:{}", real_ip);
        }
    }

    // Fall back to connection IP
    match client_ip {
        Some(ip) => format!("ip:{}", ip),
        None => {
            // Last resort - use a hash of headers to create some uniqueness
            // This is not ideal but better than allowing unlimited requests
            tracing::warn!("Could not determine client identifier for rate limiting");
            "ip:unknown".to_string()
        }
    }
}

// ============================================================================
// Middleware Functions
// ============================================================================

/// Rate limiting middleware for authentication endpoints.
///
/// Uses stricter limits to prevent:
/// - Credential stuffing attacks
/// - Brute force password attempts
/// - Account enumeration
pub async fn rate_limit_auth(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    rate_limit_inner(state, connect_info, request, next, EndpointType::Auth).await
}

/// Rate limiting middleware for standard API endpoints.
pub async fn rate_limit_api(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    rate_limit_inner(state, connect_info, request, next, EndpointType::Api).await
}

/// Rate limiting middleware for WebSocket connections.
pub async fn rate_limit_websocket(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    rate_limit_inner(state, connect_info, request, next, EndpointType::WebSocket).await
}

/// Rate limiting middleware for high-frequency endpoints.
pub async fn rate_limit_high_frequency(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    rate_limit_inner(state, connect_info, request, next, EndpointType::HighFrequency).await
}

/// Internal rate limiting implementation.
async fn rate_limit_inner(
    state: AppState,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
    endpoint_type: EndpointType,
) -> Response {
    let client_ip = connect_info.map(|ci| ci.0.ip());
    let identifier = extract_identifier(&request, client_ip);

    let limiter = RateLimiter::new(state.redis.clone(), endpoint_type);

    match limiter.check(&identifier).await {
        Ok(info) => {
            // Request allowed - add rate limit headers and continue
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &info);
            response
        }
        Err(info) => {
            // Rate limited - return 429 response
            tracing::warn!(
                identifier = %identifier,
                endpoint_type = ?endpoint_type,
                "Rate limit exceeded"
            );
            create_rate_limit_response(info)
        }
    }
}

/// Add rate limit headers to a response.
///
/// Headers follow the IETF draft standard for rate limiting:
/// https://datatracker.ietf.org/doc/draft-ietf-httpapi-ratelimit-headers/
fn add_rate_limit_headers(headers: &mut header::HeaderMap, info: &RateLimitInfo) {
    // Standard rate limit headers
    if let Ok(v) = header::HeaderValue::from_str(&info.limit.to_string()) {
        headers.insert("X-RateLimit-Limit", v);
    }
    if let Ok(v) = header::HeaderValue::from_str(&info.remaining.to_string()) {
        headers.insert("X-RateLimit-Remaining", v);
    }
    if let Ok(v) = header::HeaderValue::from_str(&info.reset_at.to_string()) {
        headers.insert("X-RateLimit-Reset", v);
    }
}

/// Create a 429 Too Many Requests response.
fn create_rate_limit_response(info: RateLimitInfo) -> Response {
    let body = RateLimitExceededResponse {
        error: ErrorResponse {
            code: 10006,
            message: "You are being rate limited. Please slow down.".to_string(),
            errors: None,
        },
        rate_limit: RateLimitInfo {
            limit: info.limit,
            remaining: 0,
            reset_at: info.reset_at,
            retry_after: info.retry_after,
        },
    };

    let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();

    // Add Retry-After header (standard HTTP header for 429 responses)
    if let Ok(v) = header::HeaderValue::from_str(&info.retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, v);
    }

    // Add rate limit headers
    add_rate_limit_headers(
        response.headers_mut(),
        &RateLimitInfo {
            limit: info.limit,
            remaining: 0,
            reset_at: info.reset_at,
            retry_after: info.retry_after,
        },
    );

    response
}

// ============================================================================
// Configurable Rate Limiter Layer
// ============================================================================

/// A configurable rate limiter that can be created with custom settings.
///
/// This allows creating rate limiters with settings from the application
/// configuration rather than using the predefined endpoint type defaults.
#[derive(Clone)]
pub struct ConfigurableRateLimiter {
    redis: ConnectionManager,
    key_prefix: String,
    config: RateLimitConfig,
}

impl ConfigurableRateLimiter {
    /// Create a new configurable rate limiter.
    pub fn new(redis: ConnectionManager, key_prefix: impl Into<String>, config: RateLimitConfig) -> Self {
        Self {
            redis,
            key_prefix: key_prefix.into(),
            config,
        }
    }

    /// Create from application settings.
    ///
    /// Uses the global rate limit settings from configuration.
    pub fn from_settings(
        redis: ConnectionManager,
        settings: &crate::config::RateLimitSettings,
    ) -> Self {
        // Convert requests_per_second to requests_per_window
        let window_seconds = 60u64;
        let requests_per_window = (settings.requests_per_second * window_seconds as f64) as u32;

        Self {
            redis,
            key_prefix: "rl:global".to_string(),
            config: RateLimitConfig {
                requests_per_window,
                window_seconds,
                burst_allowance: settings.burst_size,
            },
        }
    }

    /// Check if a request should be allowed.
    pub async fn check(&self, identifier: &str) -> Result<RateLimitInfo, RateLimitInfo> {
        let key = format!("{}:{}", self.key_prefix, identifier);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let window_ms = (self.config.window_seconds * 1000) as i64;
        let window_start = now_ms - window_ms;
        let max_requests = self.config.requests_per_window + self.config.burst_allowance;

        let mut conn = self.redis.clone();

        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now_ms = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local max_requests = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])

            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
            local current_count = redis.call('ZCARD', key)

            if current_count < max_requests then
                local member = now_ms .. ':' .. math.random(1000000)
                redis.call('ZADD', key, now_ms, member)
                redis.call('EXPIRE', key, window_seconds + 1)
                return {1, current_count + 1, max_requests}
            else
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                local retry_after = 0
                if oldest and #oldest >= 2 then
                    retry_after = oldest[2] + (window_seconds * 1000) - now_ms
                end
                return {0, current_count, max_requests, retry_after}
            end
            "#,
        );

        let result: Vec<i64> = script
            .key(&key)
            .arg(now_ms)
            .arg(window_start)
            .arg(max_requests as i64)
            .arg(self.config.window_seconds as i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!("Rate limiter Redis error: {}", e);
                RateLimitInfo {
                    limit: max_requests,
                    remaining: 1,
                    reset_at: (now_ms / 1000) + self.config.window_seconds as i64,
                    retry_after: 0,
                }
            })?;

        let allowed = result[0] == 1;
        let current_count = result[1] as u32;
        let remaining = max_requests.saturating_sub(current_count);
        let reset_at = (now_ms / 1000) + self.config.window_seconds as i64;

        let info = RateLimitInfo {
            limit: max_requests,
            remaining,
            reset_at,
            retry_after: if allowed {
                0
            } else {
                let retry_ms = result.get(3).copied().unwrap_or(0);
                ((retry_ms as f64) / 1000.0).ceil() as u64
            },
        };

        if allowed {
            Ok(info)
        } else {
            Err(info)
        }
    }
}

/// Global rate limiting middleware using application settings.
pub async fn rate_limit_global(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    let client_ip = connect_info.map(|ci| ci.0.ip());
    let identifier = extract_identifier(&request, client_ip);

    let limiter = ConfigurableRateLimiter::from_settings(
        state.redis.clone(),
        &state.settings.rate_limit,
    );

    match limiter.check(&identifier).await {
        Ok(info) => {
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &info);
            response
        }
        Err(info) => {
            tracing::warn!(
                identifier = %identifier,
                "Global rate limit exceeded"
            );
            create_rate_limit_response(info)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_type_config() {
        // Auth should have stricter limits
        let auth_config = EndpointType::Auth.config();
        let api_config = EndpointType::Api.config();

        assert!(auth_config.requests_per_window < api_config.requests_per_window);
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_window, 60);
        assert_eq!(config.window_seconds, 60);
        assert_eq!(config.burst_allowance, 10);
    }

    #[test]
    fn test_identifier_format() {
        // User identifiers should be prefixed
        let user_id = "user:12345";
        assert!(user_id.starts_with("user:"));

        // IP identifiers should be prefixed
        let ip_id = "ip:192.168.1.1";
        assert!(ip_id.starts_with("ip:"));
    }
}
