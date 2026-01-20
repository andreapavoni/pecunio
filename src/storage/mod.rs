mod repository;

pub use repository::*;

/// SQL migration for initial schema
pub const MIGRATION_001_INITIAL: &str = include_str!("migrations/001_initial.sql");

/// SQL migration for budgets
pub const MIGRATION_002_BUDGETS: &str = include_str!("migrations/002_budgets.sql");

/// SQL migration for scheduled transfers
pub const MIGRATION_003_SCHEDULED: &str = include_str!("migrations/003_scheduled_transfers.sql");
