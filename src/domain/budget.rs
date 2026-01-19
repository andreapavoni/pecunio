use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Cents;

pub type BudgetId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeriodType {
    Weekly,
    Monthly,
    Yearly,
}

impl PeriodType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PeriodType::Weekly => "weekly",
            PeriodType::Monthly => "monthly",
            PeriodType::Yearly => "yearly",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "weekly" => Some(PeriodType::Weekly),
            "monthly" => Some(PeriodType::Monthly),
            "yearly" => Some(PeriodType::Yearly),
            _ => None,
        }
    }

    /// Get the start and end of the current period for a given timestamp.
    pub fn current_period(&self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        match self {
            PeriodType::Weekly => {
                // Week starts on Monday
                let weekday = now.weekday().num_days_from_monday();
                let start =
                    now.date_naive().and_hms_opt(0, 0, 0).unwrap() - Duration::days(weekday as i64);
                let end = start + Duration::days(7);
                (
                    DateTime::from_naive_utc_and_offset(start, Utc),
                    DateTime::from_naive_utc_and_offset(end, Utc),
                )
            }
            PeriodType::Monthly => {
                // Month starts on the 1st
                let start = now
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();
                let next_month = if now.month() == 12 {
                    now.date_naive()
                        .with_day(1)
                        .unwrap()
                        .with_year(now.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    now.date_naive()
                        .with_day(1)
                        .unwrap()
                        .with_month(now.month() + 1)
                        .unwrap()
                };
                let end = next_month.and_hms_opt(0, 0, 0).unwrap();
                (
                    DateTime::from_naive_utc_and_offset(start, Utc),
                    DateTime::from_naive_utc_and_offset(end, Utc),
                )
            }
            PeriodType::Yearly => {
                // Year starts on January 1st
                let start = now
                    .date_naive()
                    .with_month(1)
                    .unwrap()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();
                let next_year = now
                    .date_naive()
                    .with_year(now.year() + 1)
                    .unwrap()
                    .with_month(1)
                    .unwrap()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();
                (
                    DateTime::from_naive_utc_and_offset(start, Utc),
                    DateTime::from_naive_utc_and_offset(next_year, Utc),
                )
            }
        }
    }
}

impl std::fmt::Display for PeriodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: BudgetId,
    pub name: String,
    pub category: String,
    pub period_type: PeriodType,
    pub amount_cents: Cents,
    pub created_at: DateTime<Utc>,
}

impl Budget {
    pub fn new(
        name: String,
        category: String,
        period_type: PeriodType,
        amount_cents: Cents,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            category,
            period_type,
            amount_cents,
            created_at: Utc::now(),
        }
    }

    /// Get the current period for this budget.
    pub fn current_period(&self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        self.period_type.current_period(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_type_roundtrip() {
        for pt in [PeriodType::Weekly, PeriodType::Monthly, PeriodType::Yearly] {
            let s = pt.as_str();
            let parsed = PeriodType::from_str(s).unwrap();
            assert_eq!(pt, parsed);
        }
    }

    #[test]
    fn test_monthly_period() {
        // Test for January 15, 2024
        let date = DateTime::parse_from_rfc3339("2024-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (start, end) = PeriodType::Monthly.current_period(date);

        assert_eq!(start.format("%Y-%m-%d").to_string(), "2024-01-01");
        assert_eq!(end.format("%Y-%m-%d").to_string(), "2024-02-01");
    }

    #[test]
    fn test_yearly_period() {
        let date = DateTime::parse_from_rfc3339("2024-06-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (start, end) = PeriodType::Yearly.current_period(date);

        assert_eq!(start.format("%Y-%m-%d").to_string(), "2024-01-01");
        assert_eq!(end.format("%Y").to_string(), "2025");
    }
}
