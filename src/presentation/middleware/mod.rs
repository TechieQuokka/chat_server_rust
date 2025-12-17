//! Middleware
//!
//! Tower middleware for request processing.

pub mod auth;
pub mod cors;
pub mod logging;
pub mod rate_limit;
pub mod security;

pub use auth::{auth_middleware, optional_auth_middleware, AuthUser};
pub use rate_limit::{
    rate_limit_api,
    rate_limit_auth,
    rate_limit_global,
    rate_limit_high_frequency,
    rate_limit_websocket,
    ConfigurableRateLimiter,
    EndpointType,
    RateLimitConfig,
    RateLimiter,
    RateLimitInfo,
};
pub use security::{
    create_security_headers_layer,
    create_security_headers_layer_no_hsts,
    SecurityHeadersConfig,
    SecurityHeadersLayer,
};
