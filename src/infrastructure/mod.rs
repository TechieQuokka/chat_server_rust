//! Infrastructure Layer
//!
//! Contains implementations for external services including:
//! - Database repositories (PostgreSQL)
//! - Cache implementations (Redis)
//! - Metrics and observability (Prometheus)
//! - External API clients

pub mod cache;
pub mod database;
pub mod metrics;
pub mod repositories;
