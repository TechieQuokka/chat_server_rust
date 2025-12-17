//! Integration Tests Entry Point
//!
//! This file serves as the entry point for integration tests.
//! Tests are organized by module:
//! - `api/` - REST API endpoint tests
//! - `common/` - Shared test utilities

mod api;
mod common;

// Re-export common utilities for tests
pub use common::*;
