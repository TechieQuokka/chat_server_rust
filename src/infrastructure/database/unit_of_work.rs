//! Unit of Work Pattern Implementation
//!
//! Provides transactional boundaries for database operations.
//! Ensures all operations within a business transaction succeed or fail together.

use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;

use crate::shared::error::AppError;

/// Unit of Work trait for managing database transactions.
///
/// This pattern ensures that multiple repository operations can be
/// grouped into a single atomic transaction.
#[async_trait::async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Begin a new transaction.
    async fn begin(&self) -> Result<TransactionContext, AppError>;

    /// Commit the transaction.
    async fn commit(tx: TransactionContext) -> Result<(), AppError>;

    /// Rollback the transaction.
    async fn rollback(tx: TransactionContext) -> Result<(), AppError>;
}

/// Transaction context that wraps a SQLx transaction.
pub struct TransactionContext {
    tx: Transaction<'static, Postgres>,
}

impl TransactionContext {
    /// Create a new transaction context.
    pub fn new(tx: Transaction<'static, Postgres>) -> Self {
        Self { tx }
    }

    /// Get a reference to the underlying transaction for query execution.
    pub fn as_mut(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.tx
    }

    /// Commit the transaction.
    pub async fn commit(self) -> Result<(), AppError> {
        self.tx.commit().await.map_err(AppError::Database)
    }

    /// Rollback the transaction.
    pub async fn rollback(self) -> Result<(), AppError> {
        self.tx.rollback().await.map_err(AppError::Database)
    }
}

/// PostgreSQL Unit of Work implementation.
pub struct PgUnitOfWork {
    pool: Arc<PgPool>,
}

impl PgUnitOfWork {
    /// Create a new Unit of Work instance.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Create from a PgPool directly.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool: Arc::new(pool) }
    }
}

#[async_trait::async_trait]
impl UnitOfWork for PgUnitOfWork {
    async fn begin(&self) -> Result<TransactionContext, AppError> {
        let tx = self.pool.begin().await.map_err(AppError::Database)?;
        Ok(TransactionContext::new(tx))
    }

    async fn commit(tx: TransactionContext) -> Result<(), AppError> {
        tx.commit().await
    }

    async fn rollback(tx: TransactionContext) -> Result<(), AppError> {
        tx.rollback().await
    }
}

/// Execute a closure within a transaction.
///
/// This helper function handles the transaction lifecycle automatically,
/// committing on success and rolling back on error.
///
/// # Example
/// ```ignore
/// let result = with_transaction(&pool, |tx| async move {
///     // Perform multiple operations
///     user_repo.create_with_tx(&mut tx, &user).await?;
///     member_repo.add_member_with_tx(&mut tx, server_id, user.id).await?;
///     Ok(user)
/// }).await?;
/// ```
pub async fn with_transaction<F, Fut, T>(pool: &PgPool, f: F) -> Result<T, AppError>
where
    F: FnOnce(TransactionContext) -> Fut,
    Fut: std::future::Future<Output = Result<(T, TransactionContext), AppError>>,
{
    let tx = pool.begin().await.map_err(AppError::Database)?;
    let ctx = TransactionContext::new(tx);

    match f(ctx).await {
        Ok((result, ctx)) => {
            ctx.commit().await?;
            Ok(result)
        }
        Err(e) => {
            // Transaction will be automatically rolled back when dropped
            Err(e)
        }
    }
}

/// A simpler transaction wrapper using closure.
///
/// # Example
/// ```ignore
/// let user = execute_in_transaction(&pool, |tx| Box::pin(async move {
///     sqlx::query_as!(User, "INSERT INTO users ... RETURNING *", ...)
///         .fetch_one(tx)
///         .await
/// })).await?;
/// ```
pub async fn execute_in_transaction<F, T>(pool: &PgPool, f: F) -> Result<T, AppError>
where
    F: for<'c> FnOnce(&'c mut Transaction<'static, Postgres>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, sqlx::Error>> + Send + 'c>>,
{
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let result = f(&mut tx).await.map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here with a test database
}
