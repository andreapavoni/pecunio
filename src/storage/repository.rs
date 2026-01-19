use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{Cents, Transfer, TransferId, Wallet, WalletId, WalletType};

use super::MIGRATION_001_INITIAL;

/// Repository for persisting and querying wallets and transfers.
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    /// Create a new repository with the given SQLite connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Connect to a SQLite database at the given path.
    /// Creates the database file if it doesn't exist.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .context("Failed to connect to database")?;
        Ok(Self::new(pool))
    }

    /// Run database migrations.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(MIGRATION_001_INITIAL)
            .execute(&self.pool)
            .await
            .context("Failed to run migrations")?;
        Ok(())
    }

    /// Initialize a new database (connect + migrate).
    pub async fn init(database_url: &str) -> Result<Self> {
        let repo = Self::connect(database_url).await?;
        repo.migrate().await?;
        Ok(repo)
    }

    // ========================
    // Wallet operations
    // ========================

    /// Save a new wallet to the database.
    pub async fn save_wallet(&self, wallet: &Wallet) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO wallets (id, name, wallet_type, currency, allow_negative, description, created_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(wallet.id.to_string())
        .bind(&wallet.name)
        .bind(wallet.wallet_type.as_str())
        .bind(&wallet.currency)
        .bind(wallet.allow_negative)
        .bind(&wallet.description)
        .bind(wallet.created_at.to_rfc3339())
        .bind(wallet.archived_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await
        .context("Failed to save wallet")?;
        Ok(())
    }

    /// Get a wallet by ID.
    pub async fn get_wallet(&self, id: WalletId) -> Result<Option<Wallet>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, wallet_type, currency, allow_negative, description, created_at, archived_at
            FROM wallets
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch wallet")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_wallet(&row)?)),
            None => Ok(None),
        }
    }

    /// Get a wallet by name.
    pub async fn get_wallet_by_name(&self, name: &str) -> Result<Option<Wallet>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, wallet_type, currency, allow_negative, description, created_at, archived_at
            FROM wallets
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch wallet by name")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_wallet(&row)?)),
            None => Ok(None),
        }
    }

    /// List all wallets (optionally including archived).
    pub async fn list_wallets(&self, include_archived: bool) -> Result<Vec<Wallet>> {
        let query = if include_archived {
            "SELECT id, name, wallet_type, currency, allow_negative, description, created_at, archived_at FROM wallets ORDER BY name"
        } else {
            "SELECT id, name, wallet_type, currency, allow_negative, description, created_at, archived_at FROM wallets WHERE archived_at IS NULL ORDER BY name"
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list wallets")?;

        rows.iter().map(Self::row_to_wallet).collect()
    }

    /// Archive a wallet (soft delete).
    pub async fn archive_wallet(&self, id: WalletId) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE wallets SET archived_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to archive wallet")?;
        Ok(())
    }

    fn row_to_wallet(row: &sqlx::sqlite::SqliteRow) -> Result<Wallet> {
        let id_str: String = row.get("id");
        let wallet_type_str: String = row.get("wallet_type");
        let created_at_str: String = row.get("created_at");
        let archived_at_str: Option<String> = row.get("archived_at");

        Ok(Wallet {
            id: Uuid::parse_str(&id_str).context("Invalid wallet ID")?,
            name: row.get("name"),
            wallet_type: WalletType::from_str(&wallet_type_str)
                .ok_or_else(|| anyhow::anyhow!("Invalid wallet type: {}", wallet_type_str))?,
            currency: row.get("currency"),
            allow_negative: row.get::<i32, _>("allow_negative") != 0,
            description: row.get("description"),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .context("Invalid created_at timestamp")?
                .with_timezone(&Utc),
            archived_at: archived_at_str
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .context("Invalid archived_at timestamp")?
                .map(|dt| dt.with_timezone(&Utc)),
        })
    }

    // ========================
    // Transfer operations
    // ========================

    /// Save a new transfer to the database.
    /// Automatically assigns the next sequence number.
    pub async fn save_transfer(&self, transfer: &mut Transfer) -> Result<()> {
        // Get and increment sequence number atomically
        let sequence = self.next_sequence().await?;
        transfer.sequence = sequence;

        let tags_json = serde_json::to_string(&transfer.tags)?;

        sqlx::query(
            r#"
            INSERT INTO transfers (id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(transfer.id.to_string())
        .bind(transfer.sequence)
        .bind(transfer.from_wallet.to_string())
        .bind(transfer.to_wallet.to_string())
        .bind(transfer.amount_cents)
        .bind(transfer.timestamp.to_rfc3339())
        .bind(transfer.recorded_at.to_rfc3339())
        .bind(&transfer.description)
        .bind(&transfer.category)
        .bind(&tags_json)
        .bind(transfer.reverses.map(|id| id.to_string()))
        .bind(&transfer.external_ref)
        .execute(&self.pool)
        .await
        .context("Failed to save transfer")?;

        Ok(())
    }

    /// Get the next sequence number and increment the counter.
    async fn next_sequence(&self) -> Result<i64> {
        let row = sqlx::query(
            r#"
            UPDATE sequence_counter
            SET value = value + 1
            WHERE name = 'transfer_sequence'
            RETURNING value
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get next sequence number")?;

        Ok(row.get("value"))
    }

    /// Get a transfer by ID.
    pub async fn get_transfer(&self, id: TransferId) -> Result<Option<Transfer>> {
        let row = sqlx::query(
            r#"
            SELECT id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref
            FROM transfers
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch transfer")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_transfer(&row)?)),
            None => Ok(None),
        }
    }

    /// List all transfers, ordered by sequence number.
    pub async fn list_transfers(&self) -> Result<Vec<Transfer>> {
        let rows = sqlx::query(
            r#"
            SELECT id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref
            FROM transfers
            ORDER BY sequence
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list transfers")?;

        rows.iter().map(Self::row_to_transfer).collect()
    }

    /// List transfers for a specific wallet (as source or destination).
    pub async fn list_transfers_for_wallet(&self, wallet_id: WalletId) -> Result<Vec<Transfer>> {
        let rows = sqlx::query(
            r#"
            SELECT id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref
            FROM transfers
            WHERE from_wallet_id = ? OR to_wallet_id = ?
            ORDER BY sequence
            "#,
        )
        .bind(wallet_id.to_string())
        .bind(wallet_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list transfers for wallet")?;

        rows.iter().map(Self::row_to_transfer).collect()
    }

    /// Compute the balance for a wallet using SQL aggregation.
    /// This is more efficient than loading all transfers and computing in memory.
    pub async fn compute_balance(&self, wallet_id: WalletId) -> Result<Cents> {
        let wallet_id_str = wallet_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN to_wallet_id = ? THEN amount_cents ELSE 0 END), 0) -
                COALESCE(SUM(CASE WHEN from_wallet_id = ? THEN amount_cents ELSE 0 END), 0) as balance
            FROM transfers
            WHERE from_wallet_id = ? OR to_wallet_id = ?
            "#,
        )
        .bind(&wallet_id_str)
        .bind(&wallet_id_str)
        .bind(&wallet_id_str)
        .bind(&wallet_id_str)
        .fetch_one(&self.pool)
        .await
        .context("Failed to compute balance")?;

        Ok(row.get("balance"))
    }

    fn row_to_transfer(row: &sqlx::sqlite::SqliteRow) -> Result<Transfer> {
        let id_str: String = row.get("id");
        let from_wallet_str: String = row.get("from_wallet_id");
        let to_wallet_str: String = row.get("to_wallet_id");
        let timestamp_str: String = row.get("timestamp");
        let recorded_at_str: String = row.get("recorded_at");
        let tags_json: String = row.get("tags");
        let reverses_str: Option<String> = row.get("reverses");

        Ok(Transfer {
            id: Uuid::parse_str(&id_str).context("Invalid transfer ID")?,
            sequence: row.get("sequence"),
            from_wallet: Uuid::parse_str(&from_wallet_str).context("Invalid from_wallet ID")?,
            to_wallet: Uuid::parse_str(&to_wallet_str).context("Invalid to_wallet ID")?,
            amount_cents: row.get("amount_cents"),
            timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                .context("Invalid timestamp")?
                .with_timezone(&Utc),
            recorded_at: DateTime::parse_from_rfc3339(&recorded_at_str)
                .context("Invalid recorded_at")?
                .with_timezone(&Utc),
            description: row.get("description"),
            category: row.get("category"),
            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            reverses: reverses_str
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .context("Invalid reverses ID")?,
            external_ref: row.get("external_ref"),
        })
    }
}
