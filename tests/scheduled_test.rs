mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use common::{StandardWallets, parse_date, test_service};
use pecunio::domain::{RecurrencePattern, ScheduleStatus, WalletType};

#[tokio::test]
async fn test_create_scheduled_transfer() -> Result<()> {
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
            "Salary".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create scheduled transfer
    let start = parse_date("2024-02-01");
    let scheduled = service
        .create_scheduled_transfer(
            "MonthlySalary".to_string(),
            "Salary",
            "Checking",
            500000, // 5000.00
            RecurrencePattern::Monthly,
            start,
            None,
            Some("Monthly salary".to_string()),
            Some("income".to_string()),
        )
        .await?;

    assert_eq!(scheduled.name, "MonthlySalary");
    assert_eq!(scheduled.amount_cents, 500000);
    assert_eq!(scheduled.pattern, RecurrencePattern::Monthly);
    assert_eq!(scheduled.description, Some("Monthly salary".to_string()));
    assert_eq!(scheduled.category, Some("income".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_scheduled_transfers() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_with_expense_categories(&service).await?;

    // Create multiple scheduled transfers
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "MonthlyRent".to_string(),
            "Checking",
            "Rent",
            120000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            Some("housing".to_string()),
        )
        .await?;

    service
        .create_scheduled_transfer(
            "WeeklyGroceries".to_string(),
            "Checking",
            "Groceries",
            15000,
            RecurrencePattern::Weekly,
            start,
            None,
            None,
            Some("groceries".to_string()),
        )
        .await?;

    // List all
    let schedules = service.list_scheduled_transfers(false).await?;
    assert_eq!(schedules.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_pause_and_resume_scheduled_transfer() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_basic(&service).await?;

    // Create scheduled transfer
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "MonthlySavings".to_string(),
            "Checking",
            "Savings",
            50000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Pause it
    let paused = service.pause_scheduled_transfer("MonthlySavings").await?;
    assert_eq!(paused.status, ScheduleStatus::Paused);

    // Verify it's not in active list
    let active = service.list_scheduled_transfers(false).await?;
    assert_eq!(active.len(), 0);

    // But it's in the full list
    let all = service.list_scheduled_transfers(true).await?;
    assert_eq!(all.len(), 1);

    // Resume it
    let resumed = service.resume_scheduled_transfer("MonthlySavings").await?;
    assert_eq!(resumed.status, ScheduleStatus::Active);

    // Now it's back in active list
    let active = service.list_scheduled_transfers(false).await?;
    assert_eq!(active.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_delete_scheduled_transfer() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_with_expense_categories(&service).await?;

    // Create scheduled transfer
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "Rent".to_string(),
            "Checking",
            "Rent",
            100000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Verify it exists
    let schedules = service.list_scheduled_transfers(false).await?;
    assert_eq!(schedules.len(), 1);

    // Delete it
    service.delete_scheduled_transfer("Rent").await?;

    // Verify it's gone
    let schedules = service.list_scheduled_transfers(false).await?;
    assert_eq!(schedules.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_execute_scheduled_transfer() -> Result<()> {
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
            "Salary".to_string(),
            WalletType::Income,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Give initial balance by receiving past salary
    service
        .record_transfer(
            "Salary",
            "Checking",
            500000,
            parse_date("2024-01-01"),
            None,
            None,
            true,
        )
        .await?;

    // Create scheduled transfer for Feb 1
    let start = parse_date("2024-02-01");
    service
        .create_scheduled_transfer(
            "MonthlySalary".to_string(),
            "Salary",
            "Checking",
            500000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Execute it with force (since we're testing)
    let result = service
        .execute_scheduled_transfer("MonthlySalary", Some(parse_date("2024-02-01")), true)
        .await?;

    assert_eq!(result.transfer.amount_cents, 500000);
    assert_eq!(result.from_wallet_name, "Salary");
    assert_eq!(result.to_wallet_name, "Checking");

    // Verify transfer was created
    let transfers = service.list_transfers(None).await?;
    assert_eq!(transfers.len(), 2); // Initial + scheduled

    // Verify balance updated
    let balance = service.get_balance("Checking").await?;
    assert_eq!(balance.balance, 1000000); // 5000 + 5000

    Ok(())
}

#[tokio::test]
async fn test_execute_due_scheduled_transfers() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_with_expense_categories(&service).await?;

    // Give initial balance
    StandardWallets::fund_checking(&service, 1000000, parse_date("2024-01-01")).await?;

    // Create scheduled transfers with past dates (so they're due)
    let past_date = Utc::now() - Duration::days(5);

    service
        .create_scheduled_transfer(
            "PastSalary".to_string(),
            "Income",
            "Checking",
            500000,
            RecurrencePattern::Monthly,
            past_date,
            None,
            None,
            None,
        )
        .await?;

    service
        .create_scheduled_transfer(
            "PastRent".to_string(),
            "Checking",
            "Rent",
            120000,
            RecurrencePattern::Monthly,
            past_date,
            None,
            None,
            None,
        )
        .await?;

    // Execute all due transfers
    let results = service.execute_due_scheduled_transfers(Utc::now()).await?;

    // Should have executed both
    assert_eq!(results.len(), 2);

    // Verify balances updated correctly
    let checking = service.get_balance("Checking").await?;
    // Started with 10000, got +5000 from salary, -1200 for rent = 13800
    assert_eq!(checking.balance, 1380000);

    Ok(())
}

#[tokio::test]
async fn test_scheduled_transfer_with_end_date() -> Result<()> {
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
            "Subscription".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Give initial balance
    service
        .record_transfer(
            "Checking",
            "Subscription",
            1000,
            parse_date("2024-01-01"),
            None,
            None,
            true,
        )
        .await?;

    // Create scheduled transfer with end date (3 months subscription)
    let start = parse_date("2024-01-15");
    let end = parse_date("2024-03-15");

    service
        .create_scheduled_transfer(
            "TrialSubscription".to_string(),
            "Checking",
            "Subscription",
            999, // 9.99
            RecurrencePattern::Monthly,
            start,
            Some(end),
            None,
            None,
        )
        .await?;

    // Get the scheduled transfer
    let st = service.get_scheduled_transfer("TrialSubscription").await?;
    assert_eq!(st.end_date, Some(end));

    // Verify it calculates correct pending executions (should be limited by end date)
    let pending = st.pending_executions(parse_date("2024-12-31"));
    // Should only have 3 executions: Jan 15, Feb 15, Mar 15
    assert_eq!(pending.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_daily_recurrence() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_basic(&service).await?;

    // Create daily scheduled transfer
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "DailySavings".to_string(),
            "Checking",
            "Savings",
            1000, // 10.00 per day
            RecurrencePattern::Daily,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Check pending executions over 7 days
    let st = service.get_scheduled_transfer("DailySavings").await?;
    let pending = st.pending_executions(parse_date("2024-01-07"));

    // Should have 7 daily executions
    assert_eq!(pending.len(), 7);

    Ok(())
}

#[tokio::test]
async fn test_weekly_recurrence() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_with_expense_categories(&service).await?;

    // Create weekly scheduled transfer
    let start = parse_date("2024-01-01"); // Monday
    service
        .create_scheduled_transfer(
            "WeeklyGroceries".to_string(),
            "Checking",
            "Groceries",
            15000, // 150.00 per week
            RecurrencePattern::Weekly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Check pending executions over 4 weeks
    let st = service.get_scheduled_transfer("WeeklyGroceries").await?;
    let pending = st.pending_executions(parse_date("2024-01-28"));

    // Should have 4 weekly executions (Jan 1, 8, 15, 22)
    assert_eq!(pending.len(), 4);

    Ok(())
}

#[tokio::test]
async fn test_yearly_recurrence() -> Result<()> {
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
            "Insurance".to_string(),
            WalletType::Expense,
            "EUR".to_string(),
            None,
        )
        .await?;

    // Create yearly scheduled transfer
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "AnnualInsurance".to_string(),
            "Checking",
            "Insurance",
            120000, // 1200.00 per year
            RecurrencePattern::Yearly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Check pending executions over 3 years
    let st = service.get_scheduled_transfer("AnnualInsurance").await?;
    let pending = st.pending_executions(parse_date("2026-12-31"));

    // Should have 3 yearly executions (2024, 2025, 2026)
    assert_eq!(pending.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_cannot_create_duplicate_scheduled_transfer() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_basic(&service).await?;

    // Create scheduled transfer
    let start = parse_date("2024-01-01");
    service
        .create_scheduled_transfer(
            "Savings".to_string(),
            "Checking",
            "Savings",
            10000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            None,
        )
        .await?;

    // Try to create another with same name
    let result = service
        .create_scheduled_transfer(
            "Savings".to_string(),
            "Checking",
            "Savings",
            20000,
            RecurrencePattern::Monthly,
            start,
            None,
            None,
            None,
        )
        .await;

    assert!(result.is_err());

    Ok(())
}
