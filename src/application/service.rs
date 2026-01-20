use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::domain::{
    build_integrity_report, Budget, Cents, IntegrityReport, PeriodType, RecurrencePattern,
    ScheduleStatus, ScheduledTransfer, Transfer, TransferId, Wallet, WalletId, WalletType,
};
use crate::storage::Repository;

use super::AppError;

/// Application service providing high-level operations for the ledger.
/// This is the primary interface for any client (CLI, API, TUI, etc.).
pub struct LedgerService {
    repo: Repository,
}

/// Result of creating a transfer
pub struct TransferResult {
    pub transfer: Transfer,
    pub from_wallet_name: String,
    pub to_wallet_name: String,
}

/// Result of reversing a transfer
pub struct ReversalResult {
    pub reversal: Transfer,
    pub original: Transfer,
    pub from_wallet_name: String,
    pub to_wallet_name: String,
    pub is_partial: bool,
}

/// Detailed wallet information
pub struct WalletInfo {
    pub wallet: Wallet,
    pub balance: Cents,
    pub incoming_count: i64,
    pub outgoing_count: i64,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Detailed transfer information
pub struct TransferInfo {
    pub transfer: Transfer,
    pub from_wallet: Wallet,
    pub to_wallet: Wallet,
    pub total_reversed: Cents,
    pub reversals: Vec<Transfer>,
}

/// Balance entry for a wallet
pub struct BalanceEntry {
    pub wallet: Wallet,
    pub balance: Cents,
}

/// Filter for querying transfers
pub struct TransferFilter {
    pub wallet: Option<String>,
    pub category: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Budget status information
pub struct BudgetStatus {
    pub budget: Budget,
    pub spent: Cents,
    pub remaining: Cents,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Forecast result showing projected balances
pub struct ForecastResult {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub snapshots: Vec<ForecastSnapshot>,
}

/// A snapshot of wallet balances at a specific point in time
pub struct ForecastSnapshot {
    pub date: DateTime<Utc>,
    pub wallet_balances: HashMap<String, Cents>,
    pub event: Option<ForecastEvent>,
}

/// Event that caused a balance change in the forecast
pub struct ForecastEvent {
    pub scheduled_name: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: Cents,
}

impl LedgerService {
    /// Create a new ledger service with the given repository.
    pub fn new(repo: Repository) -> Self {
        Self { repo }
    }

    /// Initialize a new database at the given path.
    pub async fn init(database_path: &str) -> Result<Self, AppError> {
        let db_url = format!("sqlite:{}?mode=rwc", database_path);
        let repo = Repository::init(&db_url).await?;
        Ok(Self::new(repo))
    }

    /// Connect to an existing database.
    pub async fn connect(database_path: &str) -> Result<Self, AppError> {
        let db_url = format!("sqlite:{}", database_path);
        let repo = Repository::connect(&db_url).await?;
        Ok(Self::new(repo))
    }

    // ========================
    // Wallet operations
    // ========================

    /// Create a new wallet.
    pub async fn create_wallet(
        &self,
        name: String,
        wallet_type: WalletType,
        currency: String,
        description: Option<String>,
    ) -> Result<Wallet, AppError> {
        // Check if wallet already exists
        if self.repo.get_wallet_by_name(&name).await?.is_some() {
            return Err(AppError::WalletAlreadyExists(name));
        }

        let mut wallet = Wallet::new(name, wallet_type, currency);
        if let Some(desc) = description {
            wallet = wallet.with_description(desc);
        }

        self.repo.save_wallet(&wallet).await?;
        Ok(wallet)
    }

    /// Get a wallet by name.
    pub async fn get_wallet(&self, name: &str) -> Result<Wallet, AppError> {
        self.repo
            .get_wallet_by_name(name)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(name.to_string()))
    }

    /// Get detailed wallet information.
    pub async fn get_wallet_info(&self, name: &str) -> Result<WalletInfo, AppError> {
        let wallet = self.get_wallet(name).await?;
        let balance = self.repo.compute_balance(wallet.id).await?;
        let (incoming_count, outgoing_count) =
            self.repo.count_transfers_for_wallet(wallet.id).await?;
        let last_activity = self.repo.get_last_activity(wallet.id).await?;

        Ok(WalletInfo {
            wallet,
            balance,
            incoming_count,
            outgoing_count,
            last_activity,
        })
    }

    /// List all wallets.
    pub async fn list_wallets(&self, include_archived: bool) -> Result<Vec<Wallet>, AppError> {
        Ok(self.repo.list_wallets(include_archived).await?)
    }

    /// Archive a wallet.
    pub async fn archive_wallet(&self, name: &str) -> Result<Wallet, AppError> {
        let wallet = self.get_wallet(name).await?;
        self.repo.archive_wallet(wallet.id).await?;
        // Return the wallet (note: archived_at won't be updated in this instance)
        Ok(wallet)
    }

    /// Get balance for a single wallet.
    pub async fn get_balance(&self, name: &str) -> Result<BalanceEntry, AppError> {
        let wallet = self.get_wallet(name).await?;
        let balance = self.repo.compute_balance(wallet.id).await?;
        Ok(BalanceEntry { wallet, balance })
    }

    /// Get balances for all wallets.
    pub async fn get_all_balances(&self) -> Result<Vec<BalanceEntry>, AppError> {
        let wallets = self.repo.list_wallets(false).await?;
        let balances = self.repo.compute_all_balances().await?;

        Ok(wallets
            .into_iter()
            .map(|wallet| {
                let balance = balances.get(&wallet.id).copied().unwrap_or(0);
                BalanceEntry { wallet, balance }
            })
            .collect())
    }

    // ========================
    // Transfer operations
    // ========================

    /// Record a new transfer.
    pub async fn record_transfer(
        &self,
        from_wallet_name: &str,
        to_wallet_name: &str,
        amount_cents: Cents,
        timestamp: DateTime<Utc>,
        description: Option<String>,
        category: Option<String>,
        force: bool,
    ) -> Result<TransferResult, AppError> {
        // Validate amount
        if amount_cents <= 0 {
            return Err(AppError::InvalidAmount(
                "Amount must be positive".to_string(),
            ));
        }

        // Get wallets
        let from_wallet = self.get_wallet(from_wallet_name).await?;
        let to_wallet = self.get_wallet(to_wallet_name).await?;

        // Check if archived
        if from_wallet.is_archived() {
            return Err(AppError::WalletArchived(from_wallet_name.to_string()));
        }
        if to_wallet.is_archived() {
            return Err(AppError::WalletArchived(to_wallet_name.to_string()));
        }

        // Validate currencies match
        if from_wallet.currency != to_wallet.currency {
            return Err(AppError::CurrencyMismatch {
                from_currency: from_wallet.currency.clone(),
                to_currency: to_wallet.currency.clone(),
            });
        }

        // Validate balance if wallet doesn't allow negative
        if !from_wallet.allow_negative && !force {
            let current_balance = self.repo.compute_balance(from_wallet.id).await?;
            if current_balance < amount_cents {
                return Err(AppError::InsufficientFunds {
                    wallet_name: from_wallet_name.to_string(),
                    balance: current_balance,
                    required: amount_cents,
                });
            }
        }

        // Create and save transfer
        let mut transfer = Transfer::new(from_wallet.id, to_wallet.id, amount_cents, timestamp);

        if let Some(desc) = description {
            transfer = transfer.with_description(desc);
        }
        if let Some(cat) = category {
            transfer = transfer.with_category(cat);
        }

        self.repo.save_transfer(&mut transfer).await?;

        Ok(TransferResult {
            transfer,
            from_wallet_name: from_wallet.name,
            to_wallet_name: to_wallet.name,
        })
    }

    /// Get detailed transfer information.
    pub async fn get_transfer_info(&self, id: TransferId) -> Result<TransferInfo, AppError> {
        let transfer = self
            .repo
            .get_transfer(id)
            .await?
            .ok_or_else(|| AppError::TransferNotFound(id.to_string()))?;

        let from_wallet = self
            .repo
            .get_wallet(transfer.from_wallet)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(transfer.from_wallet.to_string()))?;

        let to_wallet = self
            .repo
            .get_wallet(transfer.to_wallet)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(transfer.to_wallet.to_string()))?;

        let total_reversed = self.repo.get_total_reversed(id).await?;
        let reversals = self.repo.get_reversals_for_transfer(id).await?;

        Ok(TransferInfo {
            transfer,
            from_wallet,
            to_wallet,
            total_reversed,
            reversals,
        })
    }

    /// List transfers, optionally filtered by wallet.
    pub async fn list_transfers(
        &self,
        wallet_name: Option<&str>,
    ) -> Result<Vec<Transfer>, AppError> {
        match wallet_name {
            Some(name) => {
                let wallet = self.get_wallet(name).await?;
                Ok(self.repo.list_transfers_for_wallet(wallet.id).await?)
            }
            None => Ok(self.repo.list_transfers().await?),
        }
    }

    /// List transfers with filters.
    pub async fn list_transfers_filtered(
        &self,
        filter: TransferFilter,
    ) -> Result<Vec<Transfer>, AppError> {
        // Resolve wallet name to ID if provided
        let wallet_id = if let Some(name) = &filter.wallet {
            Some(self.get_wallet(name).await?.id)
        } else {
            None
        };

        Ok(self
            .repo
            .list_transfers_filtered(
                wallet_id,
                filter.category.as_deref(),
                filter.from_date,
                filter.to_date,
                filter.limit,
            )
            .await?)
    }

    /// Reverse a transfer (full or partial).
    pub async fn reverse_transfer(
        &self,
        transfer_id: TransferId,
        amount_cents: Option<Cents>,
    ) -> Result<ReversalResult, AppError> {
        // Get original transfer
        let original = self
            .repo
            .get_transfer(transfer_id)
            .await?
            .ok_or_else(|| AppError::TransferNotFound(transfer_id.to_string()))?;

        // Get wallets for names
        let from_wallet = self
            .repo
            .get_wallet(original.from_wallet)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(original.from_wallet.to_string()))?;

        let to_wallet = self
            .repo
            .get_wallet(original.to_wallet)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(original.to_wallet.to_string()))?;

        // Determine reversal amount
        let reversal_amount = amount_cents.unwrap_or(original.amount_cents);

        // Validate reversal amount
        let already_reversed = self.repo.get_total_reversed(transfer_id).await?;
        let remaining = original.amount_cents - already_reversed;

        if reversal_amount > remaining {
            return Err(AppError::ReversalExceedsOriginal {
                original_id: transfer_id,
                original_amount: original.amount_cents,
                already_reversed,
                requested: reversal_amount,
            });
        }

        // Create the reversal
        let is_partial = reversal_amount != original.amount_cents;
        let mut reversal = if is_partial {
            original.create_partial_reversal(reversal_amount)
        } else {
            original.create_reversal()
        };

        // Inherit category from original
        if reversal.category.is_none() {
            reversal.category = original.category.clone();
        }

        self.repo.save_transfer(&mut reversal).await?;

        Ok(ReversalResult {
            reversal,
            original,
            from_wallet_name: from_wallet.name,
            to_wallet_name: to_wallet.name,
            is_partial,
        })
    }

    // ========================
    // Integrity operations
    // ========================

    /// Check ledger integrity and return a report.
    pub async fn check_integrity(&self) -> Result<IntegrityReport, AppError> {
        let stats = self.repo.get_integrity_stats().await?;
        let wallets = self.repo.list_wallets(true).await?;
        let balances = self.repo.compute_all_balances().await?;

        let report = build_integrity_report(
            &wallets,
            &balances,
            stats.wallet_count,
            stats.transfer_count,
            stats.has_sequence_gaps,
            stats.invalid_wallet_refs,
            stats.invalid_amounts,
        );

        Ok(report)
    }

    /// Get a map of wallet IDs to names (useful for display).
    pub async fn get_wallet_names(&self) -> Result<HashMap<WalletId, String>, AppError> {
        let wallets = self.repo.list_wallets(true).await?;
        Ok(wallets.into_iter().map(|w| (w.id, w.name)).collect())
    }

    // ========================
    // Budget operations
    // ========================

    /// Create a new budget.
    pub async fn create_budget(
        &self,
        name: String,
        category: String,
        amount_cents: Cents,
        period_type: PeriodType,
    ) -> Result<Budget, AppError> {
        // Check if budget already exists
        if self.repo.get_budget_by_name(&name).await?.is_some() {
            return Err(AppError::WalletAlreadyExists(name)); // Reuse error type
        }

        let budget = Budget::new(name, category, period_type, amount_cents);
        self.repo.save_budget(&budget).await?;
        Ok(budget)
    }

    /// Get a budget by name.
    pub async fn get_budget(&self, name: &str) -> Result<Budget, AppError> {
        self.repo
            .get_budget_by_name(name)
            .await?
            .ok_or_else(|| AppError::WalletNotFound(name.to_string())) // Reuse error type
    }

    /// List all budgets.
    pub async fn list_budgets(&self) -> Result<Vec<Budget>, AppError> {
        Ok(self.repo.list_budgets().await?)
    }

    /// Delete a budget.
    pub async fn delete_budget(&self, name: &str) -> Result<Budget, AppError> {
        let budget = self.get_budget(name).await?;
        self.repo.delete_budget(name).await?;
        Ok(budget)
    }

    /// Get budget status (spending vs limit for current period).
    pub async fn get_budget_status(&self, name: &str) -> Result<BudgetStatus, AppError> {
        let budget = self.get_budget(name).await?;
        let (period_start, period_end) = budget.current_period(Utc::now());

        let spent = self
            .repo
            .sum_transfers_by_category(&budget.category, period_start, period_end)
            .await?;

        let remaining = budget.amount_cents - spent;

        Ok(BudgetStatus {
            budget,
            spent,
            remaining,
            period_start,
            period_end,
        })
    }

    /// Get status for all budgets.
    pub async fn get_all_budget_statuses(&self) -> Result<Vec<BudgetStatus>, AppError> {
        let budgets = self.list_budgets().await?;
        let mut statuses = Vec::new();

        for budget in budgets {
            let (period_start, period_end) = budget.current_period(Utc::now());
            let spent = self
                .repo
                .sum_transfers_by_category(&budget.category, period_start, period_end)
                .await?;
            let remaining = budget.amount_cents - spent;

            statuses.push(BudgetStatus {
                budget,
                spent,
                remaining,
                period_start,
                period_end,
            });
        }

        Ok(statuses)
    }

    // ========================
    // Scheduled Transfer operations
    // ========================

    /// Create a new scheduled transfer.
    pub async fn create_scheduled_transfer(
        &self,
        name: String,
        from_wallet_name: &str,
        to_wallet_name: &str,
        amount_cents: Cents,
        pattern: RecurrencePattern,
        start_date: DateTime<Utc>,
        end_date: Option<DateTime<Utc>>,
        description: Option<String>,
        category: Option<String>,
    ) -> Result<ScheduledTransfer, AppError> {
        // Check if scheduled transfer already exists
        if self
            .repo
            .get_scheduled_transfer_by_name(&name)
            .await?
            .is_some()
        {
            return Err(AppError::ScheduledTransferAlreadyExists(name));
        }

        // Validate amount
        if amount_cents <= 0 {
            return Err(AppError::InvalidAmount(
                "Amount must be positive".to_string(),
            ));
        }

        // Get wallets
        let from_wallet = self.get_wallet(from_wallet_name).await?;
        let to_wallet = self.get_wallet(to_wallet_name).await?;

        // Check if archived
        if from_wallet.is_archived() {
            return Err(AppError::WalletArchived(from_wallet_name.to_string()));
        }
        if to_wallet.is_archived() {
            return Err(AppError::WalletArchived(to_wallet_name.to_string()));
        }

        // Validate currencies match
        if from_wallet.currency != to_wallet.currency {
            return Err(AppError::CurrencyMismatch {
                from_currency: from_wallet.currency.clone(),
                to_currency: to_wallet.currency.clone(),
            });
        }

        // Create scheduled transfer
        let mut scheduled = ScheduledTransfer::new(
            name,
            from_wallet.id,
            to_wallet.id,
            amount_cents,
            pattern,
            start_date,
        );

        if let Some(end) = end_date {
            scheduled = scheduled.with_end_date(end);
        }
        if let Some(desc) = description {
            scheduled = scheduled.with_description(desc);
        }
        if let Some(cat) = category {
            scheduled = scheduled.with_category(cat);
        }

        self.repo.save_scheduled_transfer(&scheduled).await?;
        Ok(scheduled)
    }

    /// Get a scheduled transfer by name.
    pub async fn get_scheduled_transfer(&self, name: &str) -> Result<ScheduledTransfer, AppError> {
        self.repo
            .get_scheduled_transfer_by_name(name)
            .await?
            .ok_or_else(|| AppError::ScheduledTransferNotFound(name.to_string()))
    }

    /// List all scheduled transfers.
    pub async fn list_scheduled_transfers(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<ScheduledTransfer>, AppError> {
        Ok(self.repo.list_scheduled_transfers(include_inactive).await?)
    }

    /// Pause a scheduled transfer.
    pub async fn pause_scheduled_transfer(
        &self,
        name: &str,
    ) -> Result<ScheduledTransfer, AppError> {
        let scheduled = self.get_scheduled_transfer(name).await?;

        self.repo
            .update_scheduled_transfer_status(scheduled.id, ScheduleStatus::Paused)
            .await?;

        // Return updated instance
        let mut updated = scheduled;
        updated.status = ScheduleStatus::Paused;
        Ok(updated)
    }

    /// Resume a scheduled transfer.
    pub async fn resume_scheduled_transfer(
        &self,
        name: &str,
    ) -> Result<ScheduledTransfer, AppError> {
        let scheduled = self.get_scheduled_transfer(name).await?;

        self.repo
            .update_scheduled_transfer_status(scheduled.id, ScheduleStatus::Active)
            .await?;

        // Return updated instance
        let mut updated = scheduled;
        updated.status = ScheduleStatus::Active;
        Ok(updated)
    }

    /// Delete a scheduled transfer.
    pub async fn delete_scheduled_transfer(
        &self,
        name: &str,
    ) -> Result<ScheduledTransfer, AppError> {
        let scheduled = self.get_scheduled_transfer(name).await?;
        self.repo.delete_scheduled_transfer(scheduled.id).await?;
        Ok(scheduled)
    }

    /// Execute a specific scheduled transfer once.
    pub async fn execute_scheduled_transfer(
        &self,
        name: &str,
        execution_date: Option<DateTime<Utc>>,
        force: bool,
    ) -> Result<TransferResult, AppError> {
        let scheduled = self.get_scheduled_transfer(name).await?;

        let now = Utc::now();

        // Check if completed
        if scheduled.status == ScheduleStatus::Completed {
            return Err(AppError::ScheduleCompleted(name.to_string()));
        }

        // Check if paused (can still force execute)
        if scheduled.status == ScheduleStatus::Paused && !force {
            return Err(AppError::ScheduleNotDue {
                name: name.to_string(),
                next_due: scheduled.next_execution_date(now).unwrap_or(now),
            });
        }

        // Determine execution date
        let exec_date = if let Some(date) = execution_date {
            date
        } else if force {
            now
        } else {
            // Check if due
            if !scheduled.is_due(now) {
                return Err(AppError::ScheduleNotDue {
                    name: name.to_string(),
                    next_due: scheduled.next_execution_date(now).unwrap_or(now),
                });
            }
            scheduled.next_execution_date(now).unwrap_or(now)
        };

        // Get wallet names for the transfer
        let from_wallet =
            self.repo
                .get_wallet(scheduled.from_wallet)
                .await?
                .ok_or(AppError::WalletNotFound(format!(
                    "Wallet ID: {}",
                    scheduled.from_wallet
                )))?;
        let to_wallet =
            self.repo
                .get_wallet(scheduled.to_wallet)
                .await?
                .ok_or(AppError::WalletNotFound(format!(
                    "Wallet ID: {}",
                    scheduled.to_wallet
                )))?;

        // Create the actual transfer
        let result = self
            .record_transfer(
                &from_wallet.name,
                &to_wallet.name,
                scheduled.amount_cents,
                exec_date,
                scheduled.description.clone(),
                scheduled.category.clone(),
                force, // Use force flag from scheduled execution
            )
            .await?;

        // Update last_executed_at
        self.repo
            .update_last_executed(scheduled.id, exec_date)
            .await?;

        // Check if we've reached the end date and mark as completed
        if let Some(end_date) = scheduled.end_date {
            if exec_date >= end_date {
                self.repo
                    .update_scheduled_transfer_status(scheduled.id, ScheduleStatus::Completed)
                    .await?;
            }
        }

        Ok(result)
    }

    /// Execute all due scheduled transfers up to the given date.
    pub async fn execute_due_scheduled_transfers(
        &self,
        up_to: DateTime<Utc>,
    ) -> Result<Vec<TransferResult>, AppError> {
        let scheduled_transfers = self.list_scheduled_transfers(false).await?;
        let mut results = Vec::new();

        for scheduled in scheduled_transfers {
            if scheduled.status != ScheduleStatus::Active {
                continue;
            }

            let pending = scheduled.pending_executions(up_to);

            for exec_date in pending {
                let result = self
                    .execute_scheduled_transfer(&scheduled.name, Some(exec_date), false)
                    .await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Forecast future balances based on scheduled transfers.
    pub async fn forecast_balances(&self, months: usize) -> Result<ForecastResult, AppError> {
        use chrono::{Datelike, Duration};

        let now = Utc::now();
        let start_date = now;
        let end_date = now + Duration::days((months * 30) as i64); // Approximate

        // Get current balances for all wallets
        let wallets = self.list_wallets(false).await?;
        let current_balances = self.repo.compute_all_balances().await?;

        // Initialize balance map with current balances
        let mut balances: HashMap<String, Cents> = HashMap::new();
        for wallet in &wallets {
            let balance = current_balances.get(&wallet.id).copied().unwrap_or(0);
            balances.insert(wallet.name.clone(), balance);
        }

        // Get all active scheduled transfers
        let scheduled_transfers = self.list_scheduled_transfers(false).await?;

        // Collect all execution events in the forecast period
        let mut events: Vec<(DateTime<Utc>, &ScheduledTransfer)> = Vec::new();

        for st in &scheduled_transfers {
            if st.status != ScheduleStatus::Active {
                continue;
            }

            // Get all pending executions within the forecast window
            let pending = st.pending_executions(end_date);
            for date in pending {
                if date > now && date <= end_date {
                    events.push((date, st));
                }
            }
        }

        // Sort events by date
        events.sort_by_key(|(date, _)| *date);

        // Create snapshots
        let mut snapshots = Vec::new();

        // Add initial snapshot (current state)
        snapshots.push(ForecastSnapshot {
            date: now,
            wallet_balances: balances.clone(),
            event: None,
        });

        // Process each event and create snapshot
        for (date, st) in events {
            // Get wallet names
            let from_wallet =
                self.repo
                    .get_wallet(st.from_wallet)
                    .await?
                    .ok_or(AppError::WalletNotFound(format!(
                        "Wallet ID: {}",
                        st.from_wallet
                    )))?;
            let to_wallet =
                self.repo
                    .get_wallet(st.to_wallet)
                    .await?
                    .ok_or(AppError::WalletNotFound(format!(
                        "Wallet ID: {}",
                        st.to_wallet
                    )))?;

            // Update balances
            *balances.entry(from_wallet.name.clone()).or_insert(0) -= st.amount_cents;
            *balances.entry(to_wallet.name.clone()).or_insert(0) += st.amount_cents;

            // Create snapshot with event
            snapshots.push(ForecastSnapshot {
                date,
                wallet_balances: balances.clone(),
                event: Some(ForecastEvent {
                    scheduled_name: st.name.clone(),
                    from_wallet: from_wallet.name.clone(),
                    to_wallet: to_wallet.name.clone(),
                    amount: st.amount_cents,
                }),
            });
        }

        // Add monthly snapshots if there are no events in that month
        let mut current_month_date = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        while current_month_date <= end_date {
            let month_start = current_month_date;
            let month_end = if current_month_date.month() == 12 {
                current_month_date
                    .date_naive()
                    .with_year(current_month_date.year() + 1)
                    .unwrap()
                    .with_month(1)
                    .unwrap()
            } else {
                current_month_date
                    .date_naive()
                    .with_month(current_month_date.month() + 1)
                    .unwrap()
            }
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

            // Check if we already have a snapshot for this month
            let has_snapshot = snapshots
                .iter()
                .any(|s| s.date >= month_start && s.date < month_end);

            if !has_snapshot && month_end > now {
                // Add end-of-month snapshot
                snapshots.push(ForecastSnapshot {
                    date: month_end - Duration::days(1),
                    wallet_balances: balances.clone(),
                    event: None,
                });
            }

            current_month_date = month_end;
        }

        // Sort snapshots by date
        snapshots.sort_by_key(|s| s.date);

        Ok(ForecastResult {
            start_date,
            end_date,
            snapshots,
        })
    }
}
