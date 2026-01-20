CREATE TABLE IF NOT EXISTS scheduled_transfers (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL UNIQUE,
    from_wallet_id    TEXT NOT NULL,
    to_wallet_id      TEXT NOT NULL,
    amount_cents      INTEGER NOT NULL CHECK (amount_cents > 0),
    pattern           TEXT NOT NULL CHECK (pattern IN ('daily', 'weekly', 'monthly', 'yearly')),
    start_date        TEXT NOT NULL,
    end_date          TEXT,
    last_executed_at  TEXT,
    description       TEXT,
    category          TEXT,
    status            TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed')),
    created_at        TEXT NOT NULL,
    FOREIGN KEY (from_wallet_id) REFERENCES wallets(id),
    FOREIGN KEY (to_wallet_id) REFERENCES wallets(id)
);

CREATE INDEX idx_scheduled_status ON scheduled_transfers(status);
CREATE INDEX idx_scheduled_pattern ON scheduled_transfers(pattern);
CREATE INDEX idx_scheduled_next ON scheduled_transfers(last_executed_at, start_date);
