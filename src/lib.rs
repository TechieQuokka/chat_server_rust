//! # Chat Server Library
//!
//! This crate provides a Discord-compatible chat server with:
//! - RESTful HTTP API endpoints
//! - WebSocket gateway for real-time communication
//! - PostgreSQL for persistent storage
//! - Redis for caching and pub/sub messaging
//!
//! ## Architecture
//!
//! The crate follows Clean Architecture principles:
//!
//! - **Domain Layer**: Core business entities and repository traits
//! - **Application Layer**: Business logic services and DTOs
//! - **Infrastructure Layer**: Database, cache, and external service implementations
//! - **Presentation Layer**: HTTP handlers and WebSocket gateway
//!
//! ## Module Structure
//!
//! ```text
//! chat_server/
//! +-- config/        Configuration management
//! +-- domain/        Domain entities, value objects, and traits
//! +-- application/   Application services and DTOs
//! +-- infrastructure/ Database and cache implementations
//! +-- presentation/  HTTP routes and WebSocket handlers
//! +-- shared/        Common utilities (errors, snowflake IDs)
//! ```

// Configuration module
pub mod config;

// Domain layer - Core business logic
pub mod domain;

// Application layer - Business services
pub mod application;

// Infrastructure layer - External implementations
pub mod infrastructure;

// Presentation layer - HTTP and WebSocket handlers
pub mod presentation;

// Shared utilities
pub mod shared;

// Application startup and state management
pub mod startup;

// Telemetry and observability
pub mod telemetry;
