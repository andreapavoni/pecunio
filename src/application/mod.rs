//! Application layer - use cases and orchestration.
//!
//! This module provides the `LedgerService` which is the primary interface
//! for interacting with the ledger. All business logic and validation lives here,
//! making it easy to build different interfaces (CLI, API, TUI) on top.

pub mod error;
pub mod reporting;
pub mod service;

pub use error::*;
pub use reporting::*;
pub use service::*;
