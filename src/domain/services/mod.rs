//! # Domain Services
//!
//! Domain services encapsulate complex business logic that doesn't naturally
//! belong to a single entity. These services operate on domain entities and
//! implement core business rules.
//!
//! ## Services
//!
//! - **PermissionService**: Permission calculation and validation
//! - **InviteService**: Guild invite generation and validation
//! - **MessageValidationService**: Message content validation rules

mod permission_service;

pub use permission_service::*;
