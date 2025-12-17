//! # Domain Layer
//!
//! The domain layer contains the core business logic of the chat server.
//! It is independent of any external frameworks or infrastructure concerns.
//!
//! ## Structure
//!
//! - **entities**: Core domain entities (User, Guild, Channel, Message, etc.)
//! - **value_objects**: Immutable value types (Snowflake, Permissions, etc.)
//! - **services**: Domain services for complex business logic
//!
//! ## Design Principles
//!
//! - No dependencies on infrastructure or presentation layers
//! - Pure business logic and domain rules
//! - Repository traits define data access contracts
//! - Entities encapsulate domain behavior

pub mod entities;
pub mod services;
pub mod value_objects;

// Re-export commonly used types
pub use entities::*;
pub use value_objects::*;
