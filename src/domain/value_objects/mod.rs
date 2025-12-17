//! # Domain Value Objects
//!
//! Immutable value types that represent domain concepts without identity.
//!
//! ## Value Objects
//!
//! - **Snowflake**: Discord-style unique ID with embedded timestamp
//! - **Permissions**: 64-bit permission bitfield with helper methods
//! - **Color**: RGB color representation for roles and embeds

mod snowflake;
mod permissions;

pub use snowflake::*;
pub use permissions::*;
