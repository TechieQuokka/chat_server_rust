//! CORS Middleware Configuration
//!
//! Provides secure CORS configuration. Unlike common patterns that fallback to
//! allowing any origin when configuration is empty, this implementation defaults
//! to a restrictive policy (no cross-origin requests allowed) unless explicitly
//! configured.

use axum::http::{header, Method};
use tower_http::cors::CorsLayer;

use crate::config::CorsSettings;

/// Create CORS layer from settings.
///
/// # Security
///
/// If no valid origins are configured, this returns a restrictive CORS policy
/// that blocks all cross-origin requests. This prevents accidental exposure
/// of the API to cross-origin requests when misconfigured.
///
/// To allow specific origins, configure them in `cors.allowed_origins` setting.
pub fn create_cors_layer(settings: &CorsSettings) -> CorsLayer {
    let origins: Vec<_> = settings
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    if origins.is_empty() {
        // SECURITY: Do NOT fallback to Any origin - this would allow CSRF attacks.
        // Instead, return a restrictive policy that only allows same-origin requests.
        tracing::warn!(
            "No valid CORS origins configured. Cross-origin requests will be blocked. \
             Configure 'cors.allowed_origins' to allow specific origins."
        );
        CorsLayer::new()
            // No allow_origin = no cross-origin requests allowed
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
    } else {
        tracing::info!(
            origins = ?settings.allowed_origins,
            "CORS configured with {} allowed origins",
            origins.len()
        );
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::ORIGIN,
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(3600)) // 1 hour cache
    }
}
