use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type WalletId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletType {
    /// Bank accounts, cash, investments - assets you own
    Asset,
    /// Credit cards, loans - debts you owe
    Liability,
    /// External sources of money (employers, interest, etc.)
    Income,
    /// External destinations for money (merchants, bills, etc.)
    Expense,
    /// Opening balances, adjustments
    Equity,
}

impl WalletType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WalletType::Asset => "asset",
            WalletType::Liability => "liability",
            WalletType::Income => "income",
            WalletType::Expense => "expense",
            WalletType::Equity => "equity",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "asset" => Some(WalletType::Asset),
            "liability" => Some(WalletType::Liability),
            "income" => Some(WalletType::Income),
            "expense" => Some(WalletType::Expense),
            "equity" => Some(WalletType::Equity),
            _ => None,
        }
    }

    /// Returns true if this wallet type represents an external entity
    /// (money entering or leaving the system)
    pub fn is_external(&self) -> bool {
        matches!(
            self,
            WalletType::Income | WalletType::Expense | WalletType::Equity
        )
    }
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: WalletId,
    pub name: String,
    pub wallet_type: WalletType,
    pub currency: String,
    pub allow_negative: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl Wallet {
    pub fn new(name: String, wallet_type: WalletType, currency: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            wallet_type,
            currency,
            // Only Assets cannot go negative. All external wallets (Income, Expense, Equity)
            // and Liabilities can have negative balances.
            allow_negative: !matches!(wallet_type, WalletType::Asset),
            description: None,
            created_at: Utc::now(),
            archived_at: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_allow_negative(mut self, allow: bool) -> Self {
        self.allow_negative = allow;
        self
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn is_external(&self) -> bool {
        self.wallet_type.is_external()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_type_roundtrip() {
        for wt in [
            WalletType::Asset,
            WalletType::Liability,
            WalletType::Income,
            WalletType::Expense,
            WalletType::Equity,
        ] {
            let s = wt.as_str();
            let parsed = WalletType::from_str(s).unwrap();
            assert_eq!(wt, parsed);
        }
    }

    #[test]
    fn test_liability_allows_negative_by_default() {
        let wallet = Wallet::new("Credit Card".into(), WalletType::Liability, "EUR".into());
        assert!(wallet.allow_negative);
    }

    #[test]
    fn test_asset_disallows_negative_by_default() {
        let wallet = Wallet::new("Checking".into(), WalletType::Asset, "EUR".into());
        assert!(!wallet.allow_negative);
    }

    #[test]
    fn test_external_wallets_allow_negative_by_default() {
        // Income, Expense, and Equity wallets represent external entities
        // and should allow negative balances
        let income = Wallet::new("Salary".into(), WalletType::Income, "EUR".into());
        assert!(income.allow_negative);

        let expense = Wallet::new("Groceries".into(), WalletType::Expense, "EUR".into());
        assert!(expense.allow_negative);

        let equity = Wallet::new("Opening Balance".into(), WalletType::Equity, "EUR".into());
        assert!(equity.allow_negative);
    }

    #[test]
    fn test_external_wallet_types() {
        assert!(!WalletType::Asset.is_external());
        assert!(!WalletType::Liability.is_external());
        assert!(WalletType::Income.is_external());
        assert!(WalletType::Expense.is_external());
        assert!(WalletType::Equity.is_external());
    }
}
