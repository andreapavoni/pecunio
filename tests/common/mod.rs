// Allow dead_code because these helpers are used across different test files
// which are compiled separately
#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use pecunio::application::LedgerService;
use pecunio::domain::WalletType;
use tempfile::TempDir;

/// Helper to create a test service with a temporary database
pub async fn test_service() -> Result<(LedgerService, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let service = LedgerService::init(db_path.to_str().unwrap()).await?;
    Ok((service, temp_dir))
}

/// Helper to parse a date string into DateTime<Utc>
pub fn parse_date(date_str: &str) -> DateTime<Utc> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

/// Test fixture: Standard wallet setup
pub struct StandardWallets;

impl StandardWallets {
    /// Create basic wallet set: Checking, Savings, Income, Expense
    pub async fn create_basic(service: &LedgerService) -> Result<()> {
        service
            .create_wallet("Checking".into(), WalletType::Asset, "EUR".into(), None)
            .await?;
        service
            .create_wallet("Savings".into(), WalletType::Asset, "EUR".into(), None)
            .await?;
        service
            .create_wallet("Income".into(), WalletType::Income, "EUR".into(), None)
            .await?;
        service
            .create_wallet("Expense".into(), WalletType::Expense, "EUR".into(), None)
            .await?;
        Ok(())
    }

    /// Create basic wallets plus expense category wallets
    pub async fn create_with_expense_categories(service: &LedgerService) -> Result<()> {
        Self::create_basic(service).await?;
        service
            .create_wallet("Groceries".into(), WalletType::Expense, "EUR".into(), None)
            .await?;
        service
            .create_wallet("Dining".into(), WalletType::Expense, "EUR".into(), None)
            .await?;
        service
            .create_wallet(
                "Entertainment".into(),
                WalletType::Expense,
                "EUR".into(),
                None,
            )
            .await?;
        service
            .create_wallet("Rent".into(), WalletType::Expense, "EUR".into(), None)
            .await?;
        Ok(())
    }

    /// Fund checking account with initial balance from income
    pub async fn fund_checking(
        service: &LedgerService,
        amount: i64,
        date: DateTime<Utc>,
    ) -> Result<()> {
        service
            .record_transfer("Income", "Checking", amount, date, None, None, true)
            .await?;
        Ok(())
    }

    /// Fund checking account with current timestamp
    pub async fn fund_checking_now(service: &LedgerService, amount: i64) -> Result<()> {
        Self::fund_checking(service, amount, Utc::now()).await
    }
}
