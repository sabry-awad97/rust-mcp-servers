//! # Time MCP Server Core
//!
//! This module provides timezone-aware time operations for the MCP server.
//!
//! ## Features
//! - Current time queries for any IANA timezone
//! - Time conversion between timezones  
//! - Automatic DST handling
//! - Local timezone detection
//!
//! ## Modules
//! - `error`: Custom error types and error handling
//! - `models`: Data structures for requests and responses
//! - `provider`: Core timezone operations and time calculations
//! - `utils`: Helper functions for formatting and calculations

pub mod error;
pub mod models;
pub mod provider;
pub mod utils;
