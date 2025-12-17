//! Guild Repository Implementation
//!
//! PostgreSQL implementation of guild operations.

use sqlx::PgPool;

/// PostgreSQL guild repository
pub struct PgGuildRepository {
    pool: PgPool,
}

impl PgGuildRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// TODO: Implement GuildRepository trait
