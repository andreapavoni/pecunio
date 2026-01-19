use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::domain::{
    build_integrity_report, Budget, Cents, IntegrityReport, PeriodType, Transfer, TransferId,
    Wallet, WalletId, WalletType,
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
}
