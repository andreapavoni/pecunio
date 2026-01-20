mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use common::{StandardWallets, test_service};
use pecunio::domain::RecurrencePattern;

#[tokio::test]
async fn test_forecast_with_scheduled_transfers() -> Result<()> {
    let (service, _temp) = test_service().await?;

    // Setup wallets
    StandardWallets::create_with_expense_categories(&service).await?;

    // Give initial balance
    StandardWallets::fund_checking_now(&service, 500000).await?;

    // Create scheduled transfers starting from tomorrow
    let tomorrow = Utc::now() + Duration::days(1);

    service
        .create_scheduled_transfer(
            "MonthlySalary".to_string(),
            "Income",
            "Checking",
            500000,
            RecurrencePattern::Monthly,
            tomorrow,
            None,
            None,
            None,
        )
        .await?;

    service
        .create_scheduled_transfer(
            "MonthlyRent".to_string(),
            "Checking",
            "Rent",
            120000,
            RecurrencePattern::Monthly,
            tomorrow,
            None,
            None,
            None,
        )
        .await?;

    // Forecast 3 months
    let forecast = service.forecast_balances(3).await?;

    // Should have snapshots
    assert!(!forecast.snapshots.is_empty());

    // Initial snapshot should show current balance
    let initial = &forecast.snapshots[0];
    assert_eq!(initial.wallet_balances.get("Checking"), Some(&500000));

    // Should have events for scheduled transfers
    let events_count = forecast
        .snapshots
        .iter()
        .filter(|s| s.event.is_some())
        .count();
    assert!(
        events_count >= 2,
        "Should have at least salary and rent events"
    );

    Ok(())
}
