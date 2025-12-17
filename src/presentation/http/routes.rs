//! Route Configuration
//!
//! Configures all HTTP routes for the API.

use axum::{
    middleware,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Router,
};

use super::handlers;
use crate::infrastructure::metrics;
use crate::presentation::middleware::{
    auth_middleware, create_security_headers_layer, rate_limit_api, rate_limit_auth,
    rate_limit_websocket,
};
use crate::presentation::websocket::ws_handler;
use crate::startup::AppState;

/// Create the main API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1", api_routes(state.clone()))
        // WebSocket gateway endpoint with rate limiting
        .route("/gateway", get(ws_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_websocket,
        ))
        // Health check endpoints
        .route("/health", get(handlers::health::health_check))
        .route("/health/live", get(handlers::health::liveness))
        .route("/health/ready", get(handlers::health::readiness))
        // Prometheus metrics endpoint
        .route("/metrics", get(metrics_handler))
        // Apply security headers globally to all responses
        // This layer runs last (outermost) so headers are added to all responses
        .layer(create_security_headers_layer())
        .with_state(state)
}

/// Prometheus metrics endpoint handler
async fn metrics_handler() -> impl IntoResponse {
    let metrics = metrics::gather_metrics();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics,
    )
}

/// API v1 routes
fn api_routes(state: AppState) -> Router<AppState> {
    Router::new()
        // Public routes (auth has its own stricter rate limiting)
        .nest("/auth", auth_routes(state.clone()))
        // Protected routes (require authentication)
        .nest("/users", user_routes(state.clone()))
        .nest("/guilds", guild_routes(state.clone()))
        .nest("/channels", channel_routes(state.clone()))
        // Apply API rate limiting to all API routes
        .route_layer(middleware::from_fn_with_state(state, rate_limit_api))
}

/// Authentication routes (public, with stricter rate limiting)
fn auth_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/refresh", post(handlers::auth::refresh_token))
        .route("/logout", post(handlers::auth::logout))
        // Apply stricter auth rate limiting
        .route_layer(middleware::from_fn_with_state(state, rate_limit_auth))
}

/// User routes (protected)
fn user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/@me", get(handlers::user::get_current_user))
        .route("/@me", patch(handlers::user::update_current_user))
        .route("/@me/guilds", get(handlers::user::get_user_guilds))
        .route("/:user_id", get(handlers::user::get_user))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

/// Guild routes (protected)
fn guild_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::guild::create_guild))
        .route("/:guild_id", get(handlers::guild::get_guild))
        .route("/:guild_id", patch(handlers::guild::update_guild))
        .route("/:guild_id", delete(handlers::guild::delete_guild))
        .route("/:guild_id/channels", get(handlers::guild::get_guild_channels))
        .route("/:guild_id/channels", post(handlers::channel::create_channel))
        .route("/:guild_id/members", get(handlers::guild::get_guild_members))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

/// Channel routes (protected)
fn channel_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/:channel_id", get(handlers::channel::get_channel))
        .route("/:channel_id", patch(handlers::channel::update_channel))
        .route("/:channel_id", delete(handlers::channel::delete_channel))
        .route("/:channel_id/messages", get(handlers::message::get_messages))
        .route("/:channel_id/messages", post(handlers::message::send_message))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}
