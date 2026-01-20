use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::Cents;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryReport {
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub categories: Vec<CategorySummary>,
    pub total: Cents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub category: String,
    pub total: Cents,
    pub count: i64,
    pub average: Cents,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeExpenseReport {
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub total_income: Cents,
    pub total_expense: Cents,
    pub net: Cents,
    pub income_categories: Vec<CategorySummary>,
    pub expense_categories: Vec<CategorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowReport {
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub periods: Vec<CashFlowPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowPeriod {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub inflow: Cents,
    pub outflow: Cents,
    pub net: Cents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetWorthReport {
    pub as_of: DateTime<Utc>,
    pub total_assets: Cents,
    pub total_liabilities: Cents,
    pub net_worth: Cents,
    pub assets: Vec<WalletBalance>,
    pub liabilities: Vec<WalletBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub wallet_name: String,
    pub balance: Cents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodComparisonReport {
    pub current_period: PeriodSummary,
    pub previous_period: PeriodSummary,
    pub change: Cents,
    pub change_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodSummary {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_income: Cents,
    pub total_expense: Cents,
    pub net: Cents,
}

// Helper struct for repository aggregation
#[derive(Debug, Clone)]
pub struct CategoryAggregate {
    pub category: String,
    pub count: i64,
    pub total: Cents,
    pub average: Cents,
}
