//! # Chat Server
//!
//! A Discord-compatible chat server implemented in Rust.
//!
//! This is the application entry point that initializes:
//! - Tracing/logging subsystem
//! - Configuration loading
//! - Database connection pool
//! - Redis client
//! - HTTP/WebSocket server

use anyhow::Result;
use tracing::info;

use chat_server::config::Settings;
use chat_server::startup::Application;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for structured logging
    chat_server::telemetry::init_tracing();

    info!("Starting Chat Server...");

    // Load configuration from environment and config files
    let settings = Settings::load()?;
    info!(
        host = %settings.server.host,
        port = %settings.server.port,
        environment = %settings.environment,
        "Configuration loaded"
    );

    // Build and run the application
    let application = Application::build(settings).await?;

    info!("Server ready to accept connections");
    application.run_until_stopped().await?;

    Ok(())
}
