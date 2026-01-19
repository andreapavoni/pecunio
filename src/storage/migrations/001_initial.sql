-- Initial schema for Pecunio personal finance ledger
-- This migration creates the core tables: wallets and transfers

CREATE TABLE IF NOT EXISTS wallets (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    wallet_type   TEXT NOT NULL CHECK (wallet_type IN ('asset', 'liability', 'income', 'expense', 'equity')),
    currency      TEXT NOT NULL DEFAULT 'EUR',
    allow_negative INTEGER NOT NULL DEFAULT 0,
    description   TEXT,
    created_at    TEXT NOT NULL,
    archived_at   TEXT
);

CREATE TABLE IF NOT EXISTS transfers (
    id              TEXT PRIMARY KEY,
    sequence        INTEGER NOT NULL UNIQUE,
    from_wallet_id  TEXT NOT NULL REFERENCES wallets(id),
    to_wallet_id    TEXT NOT NULL REFERENCES wallets(id),
    amount_cents    INTEGER NOT NULL CHECK (amount_cents > 0),
    timestamp       TEXT NOT NULL,
    recorded_at     TEXT NOT NULL,
    description     TEXT,
    category        TEXT,
    tags            TEXT,
    reverses        TEXT REFERENCES transfers(id),
    external_ref    TEXT
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_transfers_from ON transfers(from_wallet_id);
CREATE INDEX IF NOT EXISTS idx_transfers_to ON transfers(to_wallet_id);
CREATE INDEX IF NOT EXISTS idx_transfers_timestamp ON transfers(timestamp);
CREATE INDEX IF NOT EXISTS idx_transfers_category ON transfers(category);
CREATE INDEX IF NOT EXISTS idx_transfers_reverses ON transfers(reverses);

-- Sequence tracking table (for generating monotonic sequence numbers)
CREATE TABLE IF NOT EXISTS sequence_counter (
    name  TEXT PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0
);

-- Initialize the transfer sequence counter
INSERT OR IGNORE INTO sequence_counter (name, value) VALUES ('transfer_sequence', 0);
