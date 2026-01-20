mod common;

use anyhow::Result;
use chrono::Utc;
use common::{StandardWallets, parse_date, test_service};
use pecunio::domain::{PeriodType, WalletType};

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
    StandardWallets::create_with_expense_categories(&service).await?;
    StandardWallets::fund_checking_now(&service, 100000).await?;

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
    StandardWallets::create_with_expense_categories(&service).await?;
    StandardWallets::fund_checking_now(&service, 200000).await?;

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
