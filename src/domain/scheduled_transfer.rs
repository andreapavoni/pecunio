use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Cents, WalletId};

pub type ScheduledTransferId = Uuid;

/// Recurrence pattern for scheduled transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecurrencePattern {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrencePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecurrencePattern::Daily => "daily",
            RecurrencePattern::Weekly => "weekly",
            RecurrencePattern::Monthly => "monthly",
            RecurrencePattern::Yearly => "yearly",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Some(RecurrencePattern::Daily),
            "weekly" => Some(RecurrencePattern::Weekly),
            "monthly" => Some(RecurrencePattern::Monthly),
            "yearly" => Some(RecurrencePattern::Yearly),
            _ => None,
        }
    }
}

impl std::fmt::Display for RecurrencePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status of a scheduled transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleStatus {
    Active,
    Paused,
    Completed,
}

impl ScheduleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleStatus::Active => "active",
            ScheduleStatus::Paused => "paused",
            ScheduleStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "active" => Some(ScheduleStatus::Active),
            "paused" => Some(ScheduleStatus::Paused),
            "completed" => Some(ScheduleStatus::Completed),
            _ => None,
        }
    }
}

impl std::fmt::Display for ScheduleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A scheduled transfer that repeats according to a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTransfer {
    pub id: ScheduledTransferId,
    pub name: String,
    pub from_wallet: WalletId,
    pub to_wallet: WalletId,
    pub amount_cents: Cents,
    pub pattern: RecurrencePattern,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: ScheduleStatus,
    pub created_at: DateTime<Utc>,
}

impl ScheduledTransfer {
    /// Create a new scheduled transfer
    pub fn new(
        name: String,
        from_wallet: WalletId,
        to_wallet: WalletId,
        amount_cents: Cents,
        pattern: RecurrencePattern,
        start_date: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            from_wallet,
            to_wallet,
            amount_cents,
            pattern,
            start_date,
            end_date: None,
            last_executed_at: None,
            description: None,
            category: None,
            status: ScheduleStatus::Active,
            created_at: Utc::now(),
        }
    }

    /// Set end date (for finite recurrences)
    pub fn with_end_date(mut self, end_date: DateTime<Utc>) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set category
    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    /// Calculate the next execution date after a given reference date
    pub fn next_execution_date(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // If completed or paused, no next execution
        if self.status != ScheduleStatus::Active {
            return None;
        }

        // Start from last execution or start date
        let reference_date = self.last_executed_at.unwrap_or(self.start_date);

        // If reference is in the future, return it
        if reference_date > now {
            return Some(reference_date);
        }

        // Calculate next occurrence based on pattern
        let next = match self.pattern {
            RecurrencePattern::Daily => reference_date + Duration::days(1),
            RecurrencePattern::Weekly => reference_date + Duration::days(7),
            RecurrencePattern::Monthly => self.add_one_month(reference_date),
            RecurrencePattern::Yearly => self.add_one_year(reference_date),
        };

        // Check if we've passed the end date
        if let Some(end_date) = self.end_date {
            if next > end_date {
                return None;
            }
        }

        Some(next)
    }

    /// Check if this scheduled transfer is due for execution
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        if self.status != ScheduleStatus::Active {
            return false;
        }

        // Get the reference date (last execution or start date)
        let reference_date = self.last_executed_at.unwrap_or(self.start_date);

        // If we've never executed and start_date is in the past, it's due
        if self.last_executed_at.is_none() && self.start_date <= now {
            return true;
        }

        // Calculate next execution date and check if it's due
        if let Some(next_date) = self.next_execution_date(reference_date) {
            return next_date <= now;
        }

        false
    }

    /// Get all pending execution dates between last_executed_at (or start_date) and now
    pub fn pending_executions(&self, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
        if self.status != ScheduleStatus::Active {
            return vec![];
        }

        let mut executions = Vec::new();
        let mut current = self.last_executed_at.unwrap_or(self.start_date);

        // If start date is in the future, no pending executions
        if current > now {
            return vec![];
        }

        // Add first execution if we're at or past start date
        if self.last_executed_at.is_none() && self.start_date <= now {
            executions.push(self.start_date);
            current = self.start_date;
        }

        // Calculate subsequent executions
        loop {
            let next = match self.pattern {
                RecurrencePattern::Daily => current + Duration::days(1),
                RecurrencePattern::Weekly => current + Duration::days(7),
                RecurrencePattern::Monthly => self.add_one_month(current),
                RecurrencePattern::Yearly => self.add_one_year(current),
            };

            // Stop if next execution is in the future
            if next > now {
                break;
            }

            // Stop if we've passed the end date
            if let Some(end_date) = self.end_date {
                if next > end_date {
                    break;
                }
            }

            executions.push(next);
            current = next;
        }

        executions
    }

    /// Add one month to a date, handling month-end edge cases
    fn add_one_month(&self, date: DateTime<Utc>) -> DateTime<Utc> {
        use chrono::Months;

        let naive = date.date_naive();
        let current_day = naive.day();

        // Add one month
        let next_month_first = naive
            .with_day(1)
            .unwrap()
            .checked_add_months(Months::new(1))
            .unwrap();

        // Try to use the same day, or use last day of month if it doesn't exist
        let next_date = next_month_first.with_day(current_day).unwrap_or_else(|| {
            // Day doesn't exist in next month (e.g., Jan 31 -> Feb 31)
            // Get last day of the month by going to next month and subtracting 1 day
            next_month_first
                .checked_add_months(Months::new(1))
                .unwrap()
                .pred_opt()
                .unwrap()
        });

        next_date
            .and_hms_opt(date.hour(), date.minute(), date.second())
            .unwrap()
            .and_utc()
    }

    /// Add one year to a date, handling leap year edge cases
    fn add_one_year(&self, date: DateTime<Utc>) -> DateTime<Utc> {
        let next_year = date.year() + 1;

        // Handle Feb 29 on leap years -> Feb 28 on non-leap years
        let next_date = date.date_naive().with_year(next_year).unwrap_or_else(|| {
            // Feb 29 doesn't exist in non-leap year, use Feb 28
            date.date_naive()
                .with_day(28)
                .and_then(|d| d.with_year(next_year))
                .unwrap()
        });

        next_date
            .and_hms_opt(date.hour(), date.minute(), date.second())
            .unwrap()
            .and_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_date(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("{}T10:00:00Z", s))
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_recurrence_pattern_roundtrip() {
        let patterns = vec![
            RecurrencePattern::Daily,
            RecurrencePattern::Weekly,
            RecurrencePattern::Monthly,
            RecurrencePattern::Yearly,
        ];

        for pattern in patterns {
            let s = pattern.as_str();
            let parsed = RecurrencePattern::from_str(s).unwrap();
            assert_eq!(pattern, parsed);
        }
    }

    #[test]
    fn test_schedule_status_roundtrip() {
        let statuses = vec![
            ScheduleStatus::Active,
            ScheduleStatus::Paused,
            ScheduleStatus::Completed,
        ];

        for status in statuses {
            let s = status.as_str();
            let parsed = ScheduleStatus::from_str(s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_daily_next_execution() {
        let start = parse_date("2024-01-01");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Daily,
            start,
        );

        let next = st.next_execution_date(start).unwrap();
        assert_eq!(next, start + Duration::days(1));
    }

    #[test]
    fn test_weekly_next_execution() {
        let start = parse_date("2024-01-01");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Weekly,
            start,
        );

        let next = st.next_execution_date(start).unwrap();
        assert_eq!(next, start + Duration::days(7));
    }

    #[test]
    fn test_monthly_next_execution() {
        let start = parse_date("2024-01-15");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Monthly,
            start,
        );

        let next = st.next_execution_date(start).unwrap();
        assert_eq!(next.date_naive().to_string(), "2024-02-15");
    }

    #[test]
    fn test_monthly_month_end_edge_case() {
        // Jan 31 -> Feb should give Feb 28/29
        let start = parse_date("2024-01-31");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Monthly,
            start,
        );

        let next = st.next_execution_date(start).unwrap();
        // 2024 is a leap year, so Feb has 29 days
        assert_eq!(next.date_naive().to_string(), "2024-02-29");
    }

    #[test]
    fn test_yearly_next_execution() {
        let start = parse_date("2024-06-15");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Yearly,
            start,
        );

        let next = st.next_execution_date(start).unwrap();
        assert_eq!(next.date_naive().to_string(), "2025-06-15");
    }

    #[test]
    fn test_is_due() {
        let start = parse_date("2024-01-01");
        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Daily,
            start,
        );

        // At start date, it's due
        assert!(st.is_due(start));

        // Before start date, not due
        assert!(!st.is_due(start - Duration::days(1)));

        // After start date, due
        assert!(st.is_due(start + Duration::days(1)));
    }

    #[test]
    fn test_pending_executions() {
        let start = parse_date("2024-01-01");
        let now = parse_date("2024-01-05");

        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Daily,
            start,
        );

        let pending = st.pending_executions(now);

        // Should have 5 executions: Jan 1, 2, 3, 4, 5
        assert_eq!(pending.len(), 5);
        assert_eq!(pending[0].date_naive().to_string(), "2024-01-01");
        assert_eq!(pending[4].date_naive().to_string(), "2024-01-05");
    }

    #[test]
    fn test_paused_not_due() {
        let start = parse_date("2024-01-01");
        let mut st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Daily,
            start,
        );

        st.status = ScheduleStatus::Paused;

        // Even though date has passed, paused schedules are not due
        assert!(!st.is_due(start + Duration::days(5)));
        assert_eq!(st.pending_executions(start + Duration::days(5)).len(), 0);
    }

    #[test]
    fn test_end_date_stops_execution() {
        let start = parse_date("2024-01-01");
        let end = parse_date("2024-01-03");
        let now = parse_date("2024-01-10");

        let st = ScheduledTransfer::new(
            "test".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            RecurrencePattern::Daily,
            start,
        )
        .with_end_date(end);

        let pending = st.pending_executions(now);

        // Should only have 3 executions: Jan 1, 2, 3
        assert_eq!(pending.len(), 3);
    }
}
