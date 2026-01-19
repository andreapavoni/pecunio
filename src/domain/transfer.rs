use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Cents, WalletId};

pub type TransferId = Uuid;

/// A transfer represents an atomic movement of money from one wallet to another.
/// Transfers are immutable - corrections are made via compensating transfers (reversals).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: TransferId,
    /// Monotonically increasing sequence number for ordering
    pub sequence: i64,
    /// Source wallet (balance decreases)
    pub from_wallet: WalletId,
    /// Destination wallet (balance increases)
    pub to_wallet: WalletId,
    /// Amount in cents (always positive)
    pub amount_cents: Cents,
    /// When the transaction occurred in the real world
    pub timestamp: DateTime<Utc>,
    /// When we recorded this transfer in the system
    pub recorded_at: DateTime<Utc>,
    /// Human-readable description
    pub description: Option<String>,
    /// Category for budgeting/reporting (e.g., "groceries", "utilities")
    pub category: Option<String>,
    /// Additional tags for filtering/reporting
    pub tags: Vec<String>,
    /// If this transfer is a reversal, points to the original transfer
    pub reverses: Option<TransferId>,
    /// External reference (bank transaction ID, receipt number, etc.)
    pub external_ref: Option<String>,
}

impl Transfer {
    /// Create a new transfer. Sequence number must be assigned by the repository.
    pub fn new(
        from_wallet: WalletId,
        to_wallet: WalletId,
        amount_cents: Cents,
        timestamp: DateTime<Utc>,
    ) -> Self {
        assert!(amount_cents > 0, "Transfer amount must be positive");
        Self {
            id: Uuid::new_v4(),
            sequence: 0, // Will be set by repository
            from_wallet,
            to_wallet,
            amount_cents,
            timestamp,
            recorded_at: Utc::now(),
            description: None,
            category: None,
            tags: Vec::new(),
            reverses: None,
            external_ref: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_reverses(mut self, original_id: TransferId) -> Self {
        self.reverses = Some(original_id);
        self
    }

    pub fn with_external_ref(mut self, external_ref: impl Into<String>) -> Self {
        self.external_ref = Some(external_ref.into());
        self
    }

    /// Returns true if this transfer is a reversal of another transfer
    pub fn is_reversal(&self) -> bool {
        self.reverses.is_some()
    }

    /// Create a full reversal of this transfer (swaps from/to wallets)
    pub fn create_reversal(&self) -> Self {
        Transfer::new(
            self.to_wallet,
            self.from_wallet,
            self.amount_cents,
            Utc::now(),
        )
        .with_reverses(self.id)
        .with_description(format!(
            "Reversal of: {}",
            self.description.as_deref().unwrap_or("(no description)")
        ))
    }

    /// Create a partial reversal of this transfer
    pub fn create_partial_reversal(&self, amount_cents: Cents) -> Self {
        assert!(
            amount_cents > 0 && amount_cents <= self.amount_cents,
            "Partial reversal amount must be between 0 and original amount"
        );
        Transfer::new(self.to_wallet, self.from_wallet, amount_cents, Utc::now())
            .with_reverses(self.id)
            .with_description(format!(
                "Partial reversal of: {}",
                self.description.as_deref().unwrap_or("(no description)")
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_wallet_ids() -> (WalletId, WalletId) {
        (Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_create_transfer() {
        let (from, to) = sample_wallet_ids();
        let transfer = Transfer::new(from, to, 5000, Utc::now())
            .with_description("Test transfer")
            .with_category("groceries");

        assert_eq!(transfer.amount_cents, 5000);
        assert_eq!(transfer.from_wallet, from);
        assert_eq!(transfer.to_wallet, to);
        assert_eq!(transfer.description, Some("Test transfer".to_string()));
        assert_eq!(transfer.category, Some("groceries".to_string()));
        assert!(!transfer.is_reversal());
    }

    #[test]
    fn test_create_reversal() {
        let (from, to) = sample_wallet_ids();
        let original = Transfer::new(from, to, 5000, Utc::now()).with_description("Original");

        let reversal = original.create_reversal();

        assert_eq!(reversal.from_wallet, to); // Swapped
        assert_eq!(reversal.to_wallet, from); // Swapped
        assert_eq!(reversal.amount_cents, 5000);
        assert_eq!(reversal.reverses, Some(original.id));
        assert!(reversal.is_reversal());
    }

    #[test]
    fn test_create_partial_reversal() {
        let (from, to) = sample_wallet_ids();
        let original = Transfer::new(from, to, 10000, Utc::now());

        let partial = original.create_partial_reversal(3000);

        assert_eq!(partial.amount_cents, 3000);
        assert_eq!(partial.reverses, Some(original.id));
    }

    #[test]
    #[should_panic(expected = "Transfer amount must be positive")]
    fn test_transfer_requires_positive_amount() {
        let (from, to) = sample_wallet_ids();
        Transfer::new(from, to, 0, Utc::now());
    }
}
