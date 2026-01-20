mod common;

use anyhow::Result;
use chrono::Utc;
use common::{StandardWallets, parse_date, test_service};
use pecunio::application::TransferFilter;
use pecunio::domain::WalletType;

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
    StandardWallets::create_basic(&service).await?;

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
    StandardWallets::create_with_expense_categories(&service).await?;

    // Create transfers with categories
    StandardWallets::fund_checking_now(&service, 200000).await?;

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
    StandardWallets::create_basic(&service).await?;

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
    StandardWallets::create_with_expense_categories(&service).await?;

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
