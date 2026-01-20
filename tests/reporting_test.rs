mod common;

use anyhow::Result;
use common::{parse_date, test_service};
use pecunio::domain::{PeriodType, RecurrencePattern, WalletType};

#[tokio::test]
async fn test_category_report() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Groceries".into(), WalletType::Expense, "USD".into(), None)
        .await?;
    service
        .create_wallet("Dining".into(), WalletType::Expense, "USD".into(), None)
        .await?;
    service
        .create_wallet("Income".into(), WalletType::Income, "USD".into(), None)
        .await?;

    // Fund checking account
    service
        .record_transfer(
            "Income",
            "Checking",
            500000,
            parse_date("2024-01-01"),
            None,
            None,
            false,
        )
        .await?;

    // Create transfers with categories
    service
        .record_transfer(
            "Checking",
            "Groceries",
            15000,
            parse_date("2024-01-05"),
            None,
            Some("groceries".into()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Groceries",
            20000,
            parse_date("2024-01-12"),
            None,
            Some("groceries".into()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Dining",
            5000,
            parse_date("2024-01-10"),
            None,
            Some("dining".into()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Dining",
            7500,
            parse_date("2024-01-20"),
            None,
            Some("dining".into()),
            false,
        )
        .await?;

    // Generate category report
    let from_date = parse_date("2024-01-01");
    let to_date = parse_date("2024-02-01");
    let report = service.get_category_report(from_date, to_date).await?;

    // Verify results
    assert_eq!(report.categories.len(), 2);
    assert_eq!(report.total, 47500); // 15000 + 20000 + 5000 + 7500

    // Find groceries category
    let groceries = report
        .categories
        .iter()
        .find(|c| c.category == "groceries")
        .unwrap();
    assert_eq!(groceries.total, 35000);
    assert_eq!(groceries.count, 2);
    assert_eq!(groceries.average, 17500);
    assert!((groceries.percentage - 73.68).abs() < 0.1);

    // Find dining category
    let dining = report
        .categories
        .iter()
        .find(|c| c.category == "dining")
        .unwrap();
    assert_eq!(dining.total, 12500);
    assert_eq!(dining.count, 2);

    Ok(())
}

#[tokio::test]
async fn test_income_expense_report() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Salary".into(), WalletType::Income, "USD".into(), None)
        .await?;
    service
        .create_wallet("Freelance".into(), WalletType::Income, "USD".into(), None)
        .await?;
    service
        .create_wallet("Rent".into(), WalletType::Expense, "USD".into(), None)
        .await?;
    service
        .create_wallet("Utilities".into(), WalletType::Expense, "USD".into(), None)
        .await?;

    // Record income
    service
        .record_transfer(
            "Salary",
            "Checking",
            500000,
            parse_date("2024-01-01"),
            None,
            Some("salary".into()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Freelance",
            "Checking",
            100000,
            parse_date("2024-01-15"),
            None,
            Some("freelance".into()),
            false,
        )
        .await?;

    // Record expenses
    service
        .record_transfer(
            "Checking",
            "Rent",
            120000,
            parse_date("2024-01-05"),
            None,
            Some("housing".into()),
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Utilities",
            15000,
            parse_date("2024-01-10"),
            None,
            Some("utilities".into()),
            false,
        )
        .await?;

    // Generate report
    let from_date = parse_date("2024-01-01");
    let to_date = parse_date("2024-02-01");
    let report = service
        .get_income_expense_report(from_date, to_date)
        .await?;

    // Verify
    assert_eq!(report.total_income, 600000); // 500000 + 100000
    assert_eq!(report.total_expense, 135000); // 120000 + 15000
    assert_eq!(report.net, 465000); // 600000 - 135000

    Ok(())
}

#[tokio::test]
async fn test_cashflow_report() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Income".into(), WalletType::Income, "USD".into(), None)
        .await?;
    service
        .create_wallet("Expenses".into(), WalletType::Expense, "USD".into(), None)
        .await?;

    // Month 1: Income 5000, Expense 3000
    service
        .record_transfer(
            "Income",
            "Checking",
            500000,
            parse_date("2024-01-01"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expenses",
            300000,
            parse_date("2024-01-15"),
            None,
            None,
            false,
        )
        .await?;

    // Month 2: Income 6000, Expense 4000
    service
        .record_transfer(
            "Income",
            "Checking",
            600000,
            parse_date("2024-02-01"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expenses",
            400000,
            parse_date("2024-02-15"),
            None,
            None,
            false,
        )
        .await?;

    // Generate monthly cashflow report
    let from_date = parse_date("2024-01-01");
    let to_date = parse_date("2024-03-01");
    let report = service
        .get_cashflow_report(from_date, to_date, PeriodType::Monthly)
        .await?;

    // Verify
    assert_eq!(report.periods.len(), 2);

    // January
    let jan = &report.periods[0];
    assert_eq!(jan.inflow, 500000);
    assert_eq!(jan.outflow, 300000);
    assert_eq!(jan.net, 200000);

    // February
    let feb = &report.periods[1];
    assert_eq!(feb.inflow, 600000);
    assert_eq!(feb.outflow, 400000);
    assert_eq!(feb.net, 200000);

    Ok(())
}

#[tokio::test]
async fn test_net_worth_report() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Savings".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet(
            "CreditCard".into(),
            WalletType::Liability,
            "USD".into(),
            None,
        )
        .await?;
    service
        .create_wallet("Income".into(), WalletType::Income, "USD".into(), None)
        .await?;

    // Fund accounts
    service
        .record_transfer(
            "Income",
            "Checking",
            100000,
            parse_date("2024-01-01"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Income",
            "Savings",
            50000,
            parse_date("2024-01-01"),
            None,
            None,
            false,
        )
        .await?;
    // Borrow from credit card (increases checking, increases debt)
    service
        .record_transfer(
            "CreditCard",
            "Checking",
            20000,
            parse_date("2024-01-05"),
            None,
            None,
            false,
        )
        .await?;

    // Generate net worth report
    let report = service.get_net_worth_report().await?;

    // Verify
    // Assets: Checking (100k + 20k) + Savings (50k) = 170k
    // Liabilities: CreditCard debt = 20k
    // Net worth: 170k - 20k = 150k
    assert_eq!(report.total_assets, 170000);
    assert_eq!(report.total_liabilities, 20000);
    assert_eq!(report.net_worth, 150000);

    assert_eq!(report.assets.len(), 2);
    assert_eq!(report.liabilities.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_period_comparison() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Income".into(), WalletType::Income, "USD".into(), None)
        .await?;
    service
        .create_wallet("Expenses".into(), WalletType::Expense, "USD".into(), None)
        .await?;

    // January (previous month): Income 4000, Expense 3000, Net 1000
    service
        .record_transfer(
            "Income",
            "Checking",
            400000,
            parse_date("2024-01-01"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expenses",
            300000,
            parse_date("2024-01-15"),
            None,
            None,
            false,
        )
        .await?;

    // February (current month): Income 5000, Expense 3500, Net 1500
    service
        .record_transfer(
            "Income",
            "Checking",
            500000,
            parse_date("2024-02-01"),
            None,
            None,
            false,
        )
        .await?;
    service
        .record_transfer(
            "Checking",
            "Expenses",
            350000,
            parse_date("2024-02-15"),
            None,
            None,
            false,
        )
        .await?;

    // Generate period comparison (this would compare current vs previous based on "now")
    // For testing, we can't easily control "now", so this test verifies the structure
    let report = service.get_period_comparison(PeriodType::Monthly).await?;

    // Verify structure exists
    assert!(report.current_period.period_start < report.current_period.period_end);
    assert!(report.previous_period.period_start < report.previous_period.period_end);
    assert!(report.previous_period.period_end <= report.current_period.period_start);

    Ok(())
}

#[tokio::test]
async fn test_report_with_no_data() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets but no transfers
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;

    // Generate category report with no data
    let from_date = parse_date("2024-01-01");
    let to_date = parse_date("2024-02-01");
    let report = service.get_category_report(from_date, to_date).await?;

    // Verify empty report
    assert_eq!(report.categories.len(), 0);
    assert_eq!(report.total, 0);

    // Net worth report should work with empty balances
    let net_worth = service.get_net_worth_report().await?;
    assert_eq!(net_worth.total_assets, 0);
    assert_eq!(net_worth.total_liabilities, 0);
    assert_eq!(net_worth.net_worth, 0);

    Ok(())
}

#[tokio::test]
async fn test_auto_execution_integration() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Create wallets
    service
        .create_wallet("Checking".into(), WalletType::Asset, "USD".into(), None)
        .await?;
    service
        .create_wallet("Salary".into(), WalletType::Income, "USD".into(), None)
        .await?;

    // Create a scheduled transfer that's overdue
    let past_date = parse_date("2024-01-01");

    service
        .create_scheduled_transfer(
            "MonthlySalary".into(),
            "Salary",
            "Checking",
            500000,
            RecurrencePattern::Monthly,
            past_date,
            None,
            Some("Monthly salary".into()),
            Some("income".into()),
        )
        .await?;

    // Execute due transfers (simulates auto-execution)
    let now = parse_date("2024-02-15");
    let results = service.execute_due_scheduled_transfers(now).await?;

    // Should have executed at least once (Jan and possibly Feb)
    assert!(!results.is_empty());

    // Check that transfers were created
    let balance = service.get_balance("Checking").await?;
    assert!(balance.balance > 0);

    Ok(())
}
