//! # Sleep MCP Server Core
//!
//! This module provides sleep and delay operations for the MCP server.
//!
//! ## Features
//! - Configurable sleep durations
//! - Multiple time units (seconds, milliseconds, minutes)
//! - Sleep status tracking and cancellation
//! - Progress reporting for long sleeps
//!
//! ## Modules
//! - `error`: Custom error types and error handling
//! - `models`: Data structures for requests and responses
//! - `provider`: Core sleep operations and duration handling
//! - `utils`: Helper functions for time parsing and formatting

pub mod error;
pub mod models;
pub mod provider;
pub mod utils;
