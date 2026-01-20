-- Migration 004: Reporting Performance Indexes
-- Phase 5: Add composite indexes to optimize reporting queries

-- Composite index for category-based reports with date filtering
CREATE INDEX IF NOT EXISTS idx_transfers_timestamp_category
ON transfers(timestamp, category);

-- Index for amount-based queries with date filtering
CREATE INDEX IF NOT EXISTS idx_transfers_timestamp_amount
ON transfers(timestamp, amount_cents);
