//! # Configuration Module
//!
//! This module handles application configuration loading and management.
//! Configuration can be loaded from:
//! - Environment variables (prefixed with APP__)
//! - Configuration files (config/default.toml, config/{environment}.toml)
//! - .env files (via dotenvy)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use chat_server::config::Settings;
//!
//! let settings = Settings::load()?;
//! println!("Server will listen on {}:{}", settings.server.host, settings.server.port);
//! ```

mod settings;

pub use settings::*;
