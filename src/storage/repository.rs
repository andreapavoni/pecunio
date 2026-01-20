use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{Cents, Transfer, TransferId, Wallet, WalletId};

use super::{
    MIGRATION_001_INITIAL, MIGRATION_002_BUDGETS, MIGRATION_003_SCHEDULED, MIGRATION_004_REPORTING,
};

/// Statistics for ledger integrity verification.
#[derive(Debug, Clone)]
pub struct IntegrityStats {
    pub wallet_count: i64,
    pub transfer_count: i64,
    pub has_sequence_gaps: bool,
    pub invalid_wallet_refs: i64,
    pub invalid_amounts: i64,
}

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
            .context("Failed to run migration 001")?;

        sqlx::query(MIGRATION_002_BUDGETS)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 002")?;

        sqlx::query(MIGRATION_003_SCHEDULED)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 003")?;

        sqlx::query(MIGRATION_004_REPORTING)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 004")?;

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
            wallet_type: wallet_type_str.parse().map_err(|e| {
                anyhow::anyhow!("Invalid wallet type: {}. Error: {}", wallet_type_str, e)
            })?,
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

    /// List transfers with optional filters.
    pub async fn list_transfers_filtered(
        &self,
        wallet_id: Option<WalletId>,
        category: Option<&str>,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Transfer>> {
        // Build query dynamically based on filters
        let mut query = String::from(
            "SELECT id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref FROM transfers WHERE 1=1",
        );

        // Collect all string bindings first so they live long enough
        let wallet_id_str = wallet_id.map(|id| id.to_string());
        let from_date_str = from_date.map(|dt| dt.to_rfc3339());
        let to_date_str = to_date.map(|dt| dt.to_rfc3339());

        if wallet_id.is_some() {
            query.push_str(" AND (from_wallet_id = ? OR to_wallet_id = ?)");
        }
        if category.is_some() {
            query.push_str(" AND category = ?");
        }
        if from_date.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if to_date.is_some() {
            query.push_str(" AND timestamp <= ?");
        }

        query.push_str(" ORDER BY sequence");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        // Build the query with bindings
        let mut sql_query = sqlx::query(&query);

        if let Some(ref wid_str) = wallet_id_str {
            sql_query = sql_query.bind(wid_str).bind(wid_str);
        }
        if let Some(cat) = category {
            sql_query = sql_query.bind(cat);
        }
        if let Some(ref fd_str) = from_date_str {
            sql_query = sql_query.bind(fd_str);
        }
        if let Some(ref td_str) = to_date_str {
            sql_query = sql_query.bind(td_str);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to list filtered transfers")?;

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

    /// Compute balances for all wallets in a single query.
    /// Returns a map of wallet_id -> balance. Wallets with no transfers won't be in the map (balance = 0).
    pub async fn compute_all_balances(&self) -> Result<std::collections::HashMap<WalletId, Cents>> {
        let rows = sqlx::query(
            r#"
            SELECT
                wallet_id,
                SUM(amount) as balance
            FROM (
                SELECT to_wallet_id as wallet_id, amount_cents as amount FROM transfers
                UNION ALL
                SELECT from_wallet_id as wallet_id, -amount_cents as amount FROM transfers
            )
            GROUP BY wallet_id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to compute all balances")?;

        let mut balances = std::collections::HashMap::new();
        for row in rows {
            let wallet_id_str: String = row.get("wallet_id");
            let balance: Cents = row.get("balance");
            let wallet_id = Uuid::parse_str(&wallet_id_str).context("Invalid wallet ID")?;
            balances.insert(wallet_id, balance);
        }

        Ok(balances)
    }

    /// Get all transfers that reverse a given transfer (for partial reversal tracking).
    pub async fn get_reversals_for_transfer(
        &self,
        transfer_id: TransferId,
    ) -> Result<Vec<Transfer>> {
        let rows = sqlx::query(
            r#"
            SELECT id, sequence, from_wallet_id, to_wallet_id, amount_cents, timestamp, recorded_at, description, category, tags, reverses, external_ref
            FROM transfers
            WHERE reverses = ?
            ORDER BY sequence
            "#,
        )
        .bind(transfer_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to get reversals")?;

        rows.iter().map(Self::row_to_transfer).collect()
    }

    /// Get total amount already reversed for a transfer.
    pub async fn get_total_reversed(&self, transfer_id: TransferId) -> Result<Cents> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(amount_cents), 0) as total
            FROM transfers
            WHERE reverses = ?
            "#,
        )
        .bind(transfer_id.to_string())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total reversed")?;

        Ok(row.get("total"))
    }

    /// Count transfers for a wallet (incoming and outgoing separately).
    pub async fn count_transfers_for_wallet(&self, wallet_id: WalletId) -> Result<(i64, i64)> {
        let wallet_id_str = wallet_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN to_wallet_id = ? THEN 1 ELSE 0 END), 0) as incoming,
                COALESCE(SUM(CASE WHEN from_wallet_id = ? THEN 1 ELSE 0 END), 0) as outgoing
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
        .context("Failed to count transfers")?;

        Ok((row.get("incoming"), row.get("outgoing")))
    }

    /// Get the last transfer timestamp for a wallet.
    pub async fn get_last_activity(&self, wallet_id: WalletId) -> Result<Option<DateTime<Utc>>> {
        let wallet_id_str = wallet_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT MAX(timestamp) as last_activity
            FROM transfers
            WHERE from_wallet_id = ? OR to_wallet_id = ?
            "#,
        )
        .bind(&wallet_id_str)
        .bind(&wallet_id_str)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get last activity")?;

        let last_activity_str: Option<String> = row.get("last_activity");
        match last_activity_str {
            Some(s) => Ok(Some(
                DateTime::parse_from_rfc3339(&s)
                    .context("Invalid timestamp")?
                    .with_timezone(&Utc),
            )),
            None => Ok(None),
        }
    }

    /// Get statistics for integrity checking.
    pub async fn get_integrity_stats(&self) -> Result<IntegrityStats> {
        // Count wallets
        let wallet_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM wallets")
            .fetch_one(&self.pool)
            .await?
            .get("count");

        // Count transfers
        let transfer_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM transfers")
            .fetch_one(&self.pool)
            .await?
            .get("count");

        // Check for sequence gaps
        let sequence_check = sqlx::query(
            r#"
            SELECT
                MIN(sequence) as min_seq,
                MAX(sequence) as max_seq,
                COUNT(*) as count
            FROM transfers
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let min_seq: Option<i64> = sequence_check.get("min_seq");
        let max_seq: Option<i64> = sequence_check.get("max_seq");
        let count: i64 = sequence_check.get("count");

        let has_sequence_gaps = match (min_seq, max_seq) {
            (Some(min), Some(max)) => (max - min + 1) != count,
            _ => false,
        };

        // Check for invalid wallet references
        let invalid_refs: i64 = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM transfers t
            WHERE NOT EXISTS (SELECT 1 FROM wallets w WHERE w.id = t.from_wallet_id)
               OR NOT EXISTS (SELECT 1 FROM wallets w WHERE w.id = t.to_wallet_id)
            "#,
        )
        .fetch_one(&self.pool)
        .await?
        .get("count");

        // Check for invalid amounts
        let invalid_amounts: i64 = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM transfers
            WHERE amount_cents <= 0
            "#,
        )
        .fetch_one(&self.pool)
        .await?
        .get("count");

        Ok(IntegrityStats {
            wallet_count,
            transfer_count,
            has_sequence_gaps,
            invalid_wallet_refs: invalid_refs,
            invalid_amounts,
        })
    }

    // ========================
    // Budget operations
    // ========================

    /// Save a new budget to the database.
    pub async fn save_budget(&self, budget: &crate::domain::Budget) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO budgets (id, name, category, period_type, amount_cents, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(budget.id.to_string())
        .bind(&budget.name)
        .bind(&budget.category)
        .bind(budget.period_type.as_str())
        .bind(budget.amount_cents)
        .bind(budget.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to save budget")?;
        Ok(())
    }

    /// Get a budget by name.
    pub async fn get_budget_by_name(&self, name: &str) -> Result<Option<crate::domain::Budget>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, category, period_type, amount_cents, created_at
            FROM budgets
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch budget by name")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_budget(&row)?)),
            None => Ok(None),
        }
    }

    /// List all budgets.
    pub async fn list_budgets(&self) -> Result<Vec<crate::domain::Budget>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, category, period_type, amount_cents, created_at
            FROM budgets
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list budgets")?;

        rows.iter().map(Self::row_to_budget).collect()
    }

    /// Delete a budget.
    pub async fn delete_budget(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM budgets WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .context("Failed to delete budget")?;
        Ok(())
    }

    /// Sum transfers by category within a date range.
    pub async fn sum_transfers_by_category(
        &self,
        category: &str,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<Cents> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(amount_cents), 0) as total
            FROM transfers
            WHERE category = ? AND timestamp >= ? AND timestamp < ?
            "#,
        )
        .bind(category)
        .bind(from_date.to_rfc3339())
        .bind(to_date.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to sum transfers by category")?;

        Ok(row.get("total"))
    }

    /// Aggregate transfers by category within a date range.
    /// Returns category name, count, total, and average for each category.
    pub async fn aggregate_by_category(
        &self,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<Vec<crate::application::CategoryAggregate>> {
        let rows = sqlx::query(
            r#"
            SELECT
                category,
                COUNT(*) as count,
                SUM(amount_cents) as total
            FROM transfers
            WHERE category IS NOT NULL
              AND timestamp >= ?
              AND timestamp < ?
            GROUP BY category
            ORDER BY total DESC
            "#,
        )
        .bind(from_date.to_rfc3339())
        .bind(to_date.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to aggregate transfers by category")?;

        let mut results = Vec::new();
        for row in rows {
            let count: i64 = row.get("count");
            let total: Cents = row.get("total");
            let average = if count > 0 { total / count } else { 0 };

            results.push(crate::application::CategoryAggregate {
                category: row.get("category"),
                count,
                total,
                average,
            });
        }

        Ok(results)
    }

    /// Aggregate transfers by wallet type within a date range.
    /// Returns (inflow, outflow) for each wallet type.
    pub async fn aggregate_by_wallet_type(
        &self,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<std::collections::HashMap<crate::domain::WalletType, (Cents, Cents)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                w.wallet_type,
                SUM(CASE WHEN t.to_wallet_id = w.id THEN t.amount_cents ELSE 0 END) as inflow,
                SUM(CASE WHEN t.from_wallet_id = w.id THEN t.amount_cents ELSE 0 END) as outflow
            FROM wallets w
            LEFT JOIN transfers t ON (t.from_wallet_id = w.id OR t.to_wallet_id = w.id)
            WHERE t.timestamp >= ? AND t.timestamp < ?
            GROUP BY w.wallet_type
            "#,
        )
        .bind(from_date.to_rfc3339())
        .bind(to_date.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to aggregate transfers by wallet type")?;

        let mut results = std::collections::HashMap::new();
        for row in rows {
            let wallet_type_str: String = row.get("wallet_type");
            if let Ok(wallet_type) = wallet_type_str.parse::<crate::domain::WalletType>() {
                let inflow: Cents = row.get("inflow");
                let outflow: Cents = row.get("outflow");
                results.insert(wallet_type, (inflow, outflow));
            }
        }

        Ok(results)
    }

    // ========================
    // Scheduled Transfer operations
    // ========================

    /// Save a new scheduled transfer to the database.
    pub async fn save_scheduled_transfer(
        &self,
        st: &crate::domain::ScheduledTransfer,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO scheduled_transfers (id, name, from_wallet_id, to_wallet_id, amount_cents, pattern, start_date, end_date, last_executed_at, description, category, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(st.id.to_string())
        .bind(&st.name)
        .bind(st.from_wallet.to_string())
        .bind(st.to_wallet.to_string())
        .bind(st.amount_cents)
        .bind(st.pattern.as_str())
        .bind(st.start_date.to_rfc3339())
        .bind(st.end_date.map(|dt| dt.to_rfc3339()))
        .bind(st.last_executed_at.map(|dt| dt.to_rfc3339()))
        .bind(&st.description)
        .bind(&st.category)
        .bind(st.status.as_str())
        .bind(st.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to save scheduled transfer")?;
        Ok(())
    }

    /// Get a scheduled transfer by ID.
    pub async fn get_scheduled_transfer(
        &self,
        id: crate::domain::ScheduledTransferId,
    ) -> Result<Option<crate::domain::ScheduledTransfer>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, from_wallet_id, to_wallet_id, amount_cents, pattern, start_date, end_date, last_executed_at, description, category, status, created_at
            FROM scheduled_transfers
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch scheduled transfer")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_scheduled_transfer(&row)?)),
            None => Ok(None),
        }
    }

    /// Get a scheduled transfer by name.
    pub async fn get_scheduled_transfer_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::domain::ScheduledTransfer>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, from_wallet_id, to_wallet_id, amount_cents, pattern, start_date, end_date, last_executed_at, description, category, status, created_at
            FROM scheduled_transfers
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch scheduled transfer by name")?;

        match row {
            Some(row) => Ok(Some(Self::row_to_scheduled_transfer(&row)?)),
            None => Ok(None),
        }
    }

    /// List all scheduled transfers.
    pub async fn list_scheduled_transfers(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::domain::ScheduledTransfer>> {
        let query = if include_inactive {
            "SELECT id, name, from_wallet_id, to_wallet_id, amount_cents, pattern, start_date, end_date, last_executed_at, description, category, status, created_at FROM scheduled_transfers ORDER BY name"
        } else {
            "SELECT id, name, from_wallet_id, to_wallet_id, amount_cents, pattern, start_date, end_date, last_executed_at, description, category, status, created_at FROM scheduled_transfers WHERE status = 'active' ORDER BY name"
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list scheduled transfers")?;

        rows.iter().map(Self::row_to_scheduled_transfer).collect()
    }

    /// Update the status of a scheduled transfer.
    pub async fn update_scheduled_transfer_status(
        &self,
        id: crate::domain::ScheduledTransferId,
        status: crate::domain::ScheduleStatus,
    ) -> Result<()> {
        sqlx::query("UPDATE scheduled_transfers SET status = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to update scheduled transfer status")?;
        Ok(())
    }

    /// Update the last executed timestamp of a scheduled transfer.
    pub async fn update_last_executed(
        &self,
        id: crate::domain::ScheduledTransferId,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query("UPDATE scheduled_transfers SET last_executed_at = ? WHERE id = ?")
            .bind(timestamp.to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to update last_executed_at")?;
        Ok(())
    }

    /// Delete a scheduled transfer.
    pub async fn delete_scheduled_transfer(
        &self,
        id: crate::domain::ScheduledTransferId,
    ) -> Result<()> {
        sqlx::query("DELETE FROM scheduled_transfers WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete scheduled transfer")?;
        Ok(())
    }

    fn row_to_scheduled_transfer(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<crate::domain::ScheduledTransfer> {
        let id_str: String = row.get("id");
        let from_wallet_str: String = row.get("from_wallet_id");
        let to_wallet_str: String = row.get("to_wallet_id");
        let pattern_str: String = row.get("pattern");
        let start_date_str: String = row.get("start_date");
        let end_date_str: Option<String> = row.get("end_date");
        let last_executed_str: Option<String> = row.get("last_executed_at");
        let status_str: String = row.get("status");
        let created_at_str: String = row.get("created_at");

        Ok(crate::domain::ScheduledTransfer {
            id: Uuid::parse_str(&id_str).context("Invalid scheduled transfer ID")?,
            name: row.get("name"),
            from_wallet: Uuid::parse_str(&from_wallet_str).context("Invalid from_wallet ID")?,
            to_wallet: Uuid::parse_str(&to_wallet_str).context("Invalid to_wallet ID")?,
            amount_cents: row.get("amount_cents"),
            pattern: pattern_str.parse().map_err(|e| {
                anyhow::anyhow!("Invalid recurrence pattern: {}. Error: {}", pattern_str, e)
            })?,
            start_date: DateTime::parse_from_rfc3339(&start_date_str)
                .context("Invalid start_date")?
                .with_timezone(&Utc),
            end_date: end_date_str
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .context("Invalid end_date")?
                .map(|dt| dt.with_timezone(&Utc)),
            last_executed_at: last_executed_str
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .context("Invalid last_executed_at")?
                .map(|dt| dt.with_timezone(&Utc)),
            description: row.get("description"),
            category: row.get("category"),
            status: status_str.parse().map_err(|e| {
                anyhow::anyhow!("Invalid schedule status: {}. Error: {}", status_str, e)
            })?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .context("Invalid created_at")?
                .with_timezone(&Utc),
        })
    }

    fn row_to_budget(row: &sqlx::sqlite::SqliteRow) -> Result<crate::domain::Budget> {
        let id_str: String = row.get("id");
        let period_type_str: String = row.get("period_type");
        let created_at_str: String = row.get("created_at");

        Ok(crate::domain::Budget {
            id: Uuid::parse_str(&id_str).context("Invalid budget ID")?,
            name: row.get("name"),
            category: row.get("category"),
            period_type: period_type_str.parse().map_err(|e| {
                anyhow::anyhow!("Invalid period type: {}. Error: {}", period_type_str, e)
            })?,
            amount_cents: row.get("amount_cents"),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .context("Invalid created_at timestamp")?
                .with_timezone(&Utc),
        })
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
