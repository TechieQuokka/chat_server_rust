//! CORS Middleware Configuration

use tower_http::cors::{Any, CorsLayer};

use crate::config::CorsSettings;

/// Create CORS layer from settings
pub fn create_cors_layer(settings: &CorsSettings) -> CorsLayer {
    let origins: Vec<_> = settings
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    if origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
            .max_age(std::time::Duration::from_secs(3600)) // 1 hour default
    }
}
