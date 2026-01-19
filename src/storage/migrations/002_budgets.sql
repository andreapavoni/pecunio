-- Migration 002: Add budgets table

CREATE TABLE IF NOT EXISTS budgets (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    category      TEXT NOT NULL,
    period_type   TEXT NOT NULL CHECK (period_type IN ('weekly', 'monthly', 'yearly')),
    amount_cents  INTEGER NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_budgets_category ON budgets(category);
