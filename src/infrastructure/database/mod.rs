//! Database Module
//!
//! PostgreSQL connection pool, query utilities, and transaction management.

pub mod unit_of_work;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::DatabaseSettings;

pub use unit_of_work::{
    execute_in_transaction, with_transaction, PgUnitOfWork, TransactionContext, UnitOfWork,
};

/// Create a PostgreSQL connection pool
pub async fn create_pool(settings: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(Duration::from_secs(settings.acquire_timeout))
        .connect(&settings.url)
        .await
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
