//! Application Startup
//!
//! Application building and server initialization.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;
use redis::aio::ConnectionManager;

use crate::config::Settings;
use crate::infrastructure::{database, cache};
use crate::presentation::http::routes;
use crate::presentation::middleware::{cors, logging};
use crate::presentation::websocket::gateway::Gateway;
use crate::shared::snowflake::SnowflakeGenerator;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub snowflake: Arc<SnowflakeGenerator>,
    pub gateway: Arc<Gateway>,
    pub settings: Arc<Settings>,
}

/// Application instance
pub struct Application {
    listener: TcpListener,
    router: Router,
}

impl Application {
    /// Build the application from settings
    pub async fn build(settings: Settings) -> Result<Self> {
        // Create database pool
        let db = database::create_pool(&settings.database).await?;
        tracing::info!("Database connection pool created");

        // Create Redis client
        let redis = cache::create_redis_client(&settings.redis).await?;
        tracing::info!("Redis connection established");

        // Create snowflake generator
        let snowflake = Arc::new(SnowflakeGenerator::new(
            settings.snowflake.machine_id as u64,
            0u64, // Default node_id
        ));

        // Create WebSocket gateway
        let gateway = Arc::new(Gateway::new());

        // Create app state
        let state = AppState {
            db,
            redis,
            snowflake,
            gateway,
            settings: Arc::new(settings.clone()),
        };

        // Build router with middleware
        let router = routes::create_router(state.clone())
            .layer(logging::create_trace_layer())
            .layer(cors::create_cors_layer(&settings.cors));

        // Bind to address
        let addr = SocketAddr::from(([0, 0, 0, 0], settings.server.port));
        let listener = TcpListener::bind(addr).await?;
        tracing::info!("Listening on {}", addr);

        Ok(Self { listener, router })
    }

    /// Run the server until stopped
    pub async fn run_until_stopped(self) -> Result<()> {
        axum::serve(self.listener, self.router).await?;
        Ok(())
    }

    /// Get the bound address
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
