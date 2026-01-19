mod repository;

pub use repository::*;

/// SQL migration for initial schema
pub const MIGRATION_001_INITIAL: &str = include_str!("migrations/001_initial.sql");
