use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use pecunio::application::{LedgerService, TransferFilter};
use pecunio::domain::{PeriodType, WalletType};
use tempfile::TempDir;

/// Helper to create a test service with a temporary database
async fn test_service() -> Result<(LedgerService, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let service = LedgerService::init(db_path.to_str().unwrap()).await?;
    Ok((service, temp_dir))
}

/// Helper to parse a date string into DateTime<Utc>
fn parse_date(date_str: &str) -> DateTime<Utc> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

#[tokio::test]
async fn test_date_support_for_transfers() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Salary".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Record a backdated transfer (from income into checking)
    let date = parse_date("2024-01-15");
    service
        .record_transfer("Salary", "Checking", 500000, date, None, None, true)
        .await?;

    // Verify the transfer has the correct timestamp
    let transfers = service.list_transfers(None).await?;
    assert_eq!(transfers.len(), 1);
    assert_eq!(
        transfers[0].timestamp.date_naive().to_string(),
        "2024-01-15"
    );

    Ok(())
}

#[tokio::test]
async fn test_transfer_filtering_by_date_range() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Expense".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create transfers across different dates
    service
        .record_transfer(
            "Income",
            "Checking",
            100000,
            parse_date("2024-01-05"),
            None,
            None,
            true,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expense",
            10000,
            parse_date("2024-01-10"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expense",
            20000,
            parse_date("2024-01-20"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expense",
            5000,
            parse_date("2024-02-01"),
            None,
            None,
            false,
        )
        .await?;

    // Filter by date range (January 1-31)
    let filter = TransferFilter {
        wallet: None,
        category: None,
        from_date: Some(parse_date("2024-01-01")),
        to_date: Some(parse_date("2024-01-31")),
        limit: None,
    };

    let filtered = service.list_transfers_filtered(filter).await?;
    assert_eq!(filtered.len(), 3, "Should have 3 transfers in January");

    Ok(())
}

#[tokio::test]
async fn test_transfer_filtering_by_category() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Groceries".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Entertainment".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create transfers with categories
    service
        .record_transfer("Income", "Checking", 200000, Utc::now(), None, None, true)
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            15000,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            8500,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Entertainment",
            5000,
            Utc::now(),
            None,
            Some("entertainment".to_string()),
            false,
        )
        .await?;

    // Filter by category
    let filter = TransferFilter {
        wallet: None,
        category: Some("groceries".to_string()),
        from_date: None,
        to_date: None,
        limit: None,
    };

    let filtered = service.list_transfers_filtered(filter).await?;
    assert_eq!(filtered.len(), 2, "Should have 2 grocery transfers");

    Ok(())
}

#[tokio::test]
async fn test_transfer_filtering_by_wallet() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Savings".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Expense".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create various transfers
    service
        .record_transfer("Income", "Checking", 100000, Utc::now(), None, None, true)
        .await?;
    service
        .record_transfer("Income", "Savings", 50000, Utc::now(), None, None, true)
        .await?;
    service
        .record_transfer("Checking", "Expense", 10000, Utc::now(), None, None, false)
        .await?;
    service
        .record_transfer("Savings", "Expense", 5000, Utc::now(), None, None, false)
        .await?;

    // Filter by wallet
    let filter = TransferFilter {
        wallet: Some("Checking".to_string()),
        category: None,
        from_date: None,
        to_date: None,
        limit: None,
    };

    let filtered = service.list_transfers_filtered(filter).await?;
    assert_eq!(
        filtered.len(),
        2,
        "Should have 2 transfers involving Checking"
    );

    Ok(())
}

#[tokio::test]
async fn test_combined_filters() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Groceries".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Entertainment".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create transfers
    service
        .record_transfer(
            "Income",
            "Checking",
            300000,
            parse_date("2024-01-01"),
            None,
            None,
            true,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            15000,
            parse_date("2024-01-10"),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            8500,
            parse_date("2024-01-20"),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Entertainment",
            5000,
            parse_date("2024-01-15"),
            None,
            Some("entertainment".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            20000,
            parse_date("2024-02-05"),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;

    // Combine wallet + category + date filters
    let filter = TransferFilter {
        wallet: Some("Checking".to_string()),
        category: Some("groceries".to_string()),
        from_date: Some(parse_date("2024-01-01")),
        to_date: Some(parse_date("2024-01-31")),
        limit: None,
    };

    let filtered = service.list_transfers_filtered(filter).await?;
    assert_eq!(
        filtered.len(),
        2,
        "Should have 2 grocery transfers from Checking in January"
    );

    Ok(())
}

#[tokio::test]
async fn test_budget_create_and_list() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create budgets
    service
        .create_budget(
            "groceries".to_string(),
            "groceries".to_string(),
            40000,
            PeriodType::Monthly,
        )
        .await?;
    service
        .create_budget(
            "entertainment".to_string(),
            "entertainment".to_string(),
            10000,
            PeriodType::Monthly,
        )
        .await?;

    // List budgets
    let budgets = service.list_budgets().await?;
    assert_eq!(budgets.len(), 2);

    // Verify budget details
    let grocery_budget = budgets
        .iter()
        .find(|b| b.name == "groceries")
        .expect("Should find groceries budget");
    assert_eq!(grocery_budget.category, "groceries");
    assert_eq!(grocery_budget.amount_cents, 40000);
    assert!(matches!(grocery_budget.period_type, PeriodType::Monthly));

    Ok(())
}

#[tokio::test]
async fn test_budget_status_calculation() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Groceries".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Fund checking account
    service
        .record_transfer("Income", "Checking", 100000, Utc::now(), None, None, true)
        .await?;

    // Create grocery expenses
    service
        .record_transfer(
            "Checking",
            "Groceries",
            15000,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            8550,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;

    // Create budget
    service
        .create_budget(
            "groceries".to_string(),
            "groceries".to_string(),
            40000,
            PeriodType::Monthly,
        )
        .await?;

    // Get budget status
    let status = service.get_budget_status("groceries").await?;

    assert_eq!(status.spent, 23550); // 150.00 + 85.50 = 235.50 in cents
    assert_eq!(status.remaining, 40000 - 23550);
    assert_eq!(status.budget.name, "groceries");

    Ok(())
}

#[tokio::test]
async fn test_budget_status_multiple() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Groceries".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Entertainment".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Fund account
    service
        .record_transfer("Income", "Checking", 200000, Utc::now(), None, None, true)
        .await?;

    // Create expenses
    service
        .record_transfer(
            "Checking",
            "Groceries",
            20000,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Entertainment",
            4500,
            Utc::now(),
            None,
            Some("entertainment".to_string()),
            false,
        )
        .await?;

    // Create budgets
    service
        .create_budget(
            "groceries".to_string(),
            "groceries".to_string(),
            40000,
            PeriodType::Monthly,
        )
        .await?;
    service
        .create_budget(
            "entertainment".to_string(),
            "entertainment".to_string(),
            10000,
            PeriodType::Monthly,
        )
        .await?;

    // Get all statuses
    let statuses = service.get_all_budget_statuses().await?;
    assert_eq!(statuses.len(), 2);

    let grocery_status = statuses
        .iter()
        .find(|s| s.budget.name == "groceries")
        .unwrap();
    assert_eq!(grocery_status.spent, 20000); // 200.00

    let entertainment_status = statuses
        .iter()
        .find(|s| s.budget.name == "entertainment")
        .unwrap();
    assert_eq!(entertainment_status.spent, 4500); // 45.00

    Ok(())
}

#[tokio::test]
async fn test_budget_delete() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create budget
    service
        .create_budget(
            "groceries".to_string(),
            "groceries".to_string(),
            40000,
            PeriodType::Monthly,
        )
        .await?;

    // Verify it exists
    let budgets = service.list_budgets().await?;
    assert_eq!(budgets.len(), 1);

    // Delete it
    service.delete_budget("groceries").await?;

    // Verify it's gone
    let budgets = service.list_budgets().await?;
    assert_eq!(budgets.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_budget_only_counts_current_period() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    service
        .create_wallet(
            "Checking".to_string(),
            WalletType::Asset,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Income".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;
    service
        .create_wallet(
            "Groceries".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Fund account
    service
        .record_transfer(
            "Income",
            "Checking",
            500000,
            parse_date("2024-01-01"),
            None,
            None,
            true,
        )
        .await?;

    // Create expenses in different months
    service
        .record_transfer(
            "Checking",
            "Groceries",
            30000,
            parse_date("2023-12-15"),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            15000,
            Utc::now(),
            None,
            Some("groceries".to_string()),
            false,
        )
        .await?;

    // Create budget
    service
        .create_budget(
            "groceries".to_string(),
            "groceries".to_string(),
            40000,
            PeriodType::Monthly,
        )
        .await?;

    // Get budget status - should only count current month
    let status = service.get_budget_status("groceries").await?;

    // Should only include the 150.00 from current period, not the 300.00 from last month
    assert_eq!(
        status.spent, 15000,
        "Should only count spending in current period"
    );

    Ok(())
}
