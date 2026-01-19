use std::collections::HashMap;

use super::{Cents, Transfer, Wallet, WalletId, WalletType};

/// Compute the balance for a single wallet from a list of transfers.
/// Balance = sum of incoming transfers - sum of outgoing transfers
pub fn compute_balance(wallet_id: WalletId, transfers: &[Transfer]) -> Cents {
    transfers.iter().fold(0, |balance, transfer| {
        if transfer.to_wallet == wallet_id {
            balance + transfer.amount_cents
        } else if transfer.from_wallet == wallet_id {
            balance - transfer.amount_cents
        } else {
            balance
        }
    })
}

/// Compute balances for all wallets from a list of transfers.
/// Returns a map of wallet_id -> balance
pub fn compute_all_balances(transfers: &[Transfer]) -> HashMap<WalletId, Cents> {
    let mut balances: HashMap<WalletId, Cents> = HashMap::new();

    for transfer in transfers {
        *balances.entry(transfer.from_wallet).or_insert(0) -= transfer.amount_cents;
        *balances.entry(transfer.to_wallet).or_insert(0) += transfer.amount_cents;
    }

    balances
}

/// Calculate total reversed amount for a transfer.
/// Used to validate that partial reversals don't exceed the original amount.
pub fn total_reversed_amount(original_id: super::TransferId, transfers: &[Transfer]) -> Cents {
    transfers
        .iter()
        .filter(|t| t.reverses == Some(original_id))
        .map(|t| t.amount_cents)
        .sum()
}

/// Validate that a proposed reversal doesn't exceed the original transfer amount.
pub fn validate_reversal(
    original: &Transfer,
    reversal_amount: Cents,
    all_transfers: &[Transfer],
) -> Result<(), ReversalError> {
    let already_reversed = total_reversed_amount(original.id, all_transfers);
    if already_reversed + reversal_amount > original.amount_cents {
        return Err(ReversalError::ExceedsOriginalAmount {
            original_amount: original.amount_cents,
            already_reversed,
            requested: reversal_amount,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReversalError {
    ExceedsOriginalAmount {
        original_amount: Cents,
        already_reversed: Cents,
        requested: Cents,
    },
}

impl std::fmt::Display for ReversalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReversalError::ExceedsOriginalAmount {
                original_amount,
                already_reversed,
                requested,
            } => {
                write!(
                    f,
                    "Reversal of {} cents would exceed original amount ({} cents, {} already reversed)",
                    requested, original_amount, already_reversed
                )
            }
        }
    }
}

impl std::error::Error for ReversalError {}

/// Result of an integrity check on the ledger.
#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub wallet_count: i64,
    pub transfer_count: i64,
    pub balance_by_type: HashMap<WalletType, Cents>,
    pub total_balance: Cents,
    pub is_balanced: bool,
    pub issues: Vec<IntegrityIssue>,
}

impl IntegrityReport {
    pub fn is_healthy(&self) -> bool {
        self.is_balanced && self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityIssue {
    SequenceGaps,
    InvalidWalletReferences(i64),
    InvalidAmounts(i64),
    UnbalancedLedger(Cents),
}

impl std::fmt::Display for IntegrityIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrityIssue::SequenceGaps => write!(f, "Sequence numbers have gaps"),
            IntegrityIssue::InvalidWalletReferences(count) => {
                write!(f, "{} transfers reference non-existent wallets", count)
            }
            IntegrityIssue::InvalidAmounts(count) => {
                write!(f, "{} transfers have invalid amounts (<= 0)", count)
            }
            IntegrityIssue::UnbalancedLedger(diff) => {
                write!(f, "Ledger is unbalanced by {} cents", diff)
            }
        }
    }
}

/// Build an integrity report from wallets and balances.
pub fn build_integrity_report(
    wallets: &[Wallet],
    balances: &HashMap<WalletId, Cents>,
    wallet_count: i64,
    transfer_count: i64,
    has_sequence_gaps: bool,
    invalid_wallet_refs: i64,
    invalid_amounts: i64,
) -> IntegrityReport {
    // Group balances by wallet type
    let mut balance_by_type: HashMap<WalletType, Cents> = HashMap::new();
    for wallet in wallets {
        let balance = balances.get(&wallet.id).copied().unwrap_or(0);
        *balance_by_type.entry(wallet.wallet_type).or_insert(0) += balance;
    }

    // Calculate total balance (should be 0 for a healthy ledger)
    let total_balance: Cents = balances.values().sum();
    let is_balanced = total_balance == 0;

    // Collect issues
    let mut issues = Vec::new();
    if has_sequence_gaps {
        issues.push(IntegrityIssue::SequenceGaps);
    }
    if invalid_wallet_refs > 0 {
        issues.push(IntegrityIssue::InvalidWalletReferences(invalid_wallet_refs));
    }
    if invalid_amounts > 0 {
        issues.push(IntegrityIssue::InvalidAmounts(invalid_amounts));
    }
    if !is_balanced {
        issues.push(IntegrityIssue::UnbalancedLedger(total_balance));
    }

    IntegrityReport {
        wallet_count,
        transfer_count,
        balance_by_type,
        total_balance,
        is_balanced,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn make_transfer(from: WalletId, to: WalletId, amount: Cents) -> Transfer {
        Transfer::new(from, to, amount, Utc::now())
    }

    #[test]
    fn test_compute_balance_empty() {
        let wallet = Uuid::new_v4();
        assert_eq!(compute_balance(wallet, &[]), 0);
    }

    #[test]
    fn test_compute_balance_incoming() {
        let wallet = Uuid::new_v4();
        let external = Uuid::new_v4();
        let transfers = vec![make_transfer(external, wallet, 5000)];

        assert_eq!(compute_balance(wallet, &transfers), 5000);
    }

    #[test]
    fn test_compute_balance_outgoing() {
        let wallet = Uuid::new_v4();
        let external = Uuid::new_v4();
        let transfers = vec![make_transfer(wallet, external, 3000)];

        assert_eq!(compute_balance(wallet, &transfers), -3000);
    }

    #[test]
    fn test_compute_balance_mixed() {
        let checking = Uuid::new_v4();
        let salary = Uuid::new_v4();
        let groceries = Uuid::new_v4();

        let transfers = vec![
            make_transfer(salary, checking, 5000),    // +5000
            make_transfer(checking, groceries, 1500), // -1500
            make_transfer(checking, groceries, 500),  // -500
        ];

        assert_eq!(compute_balance(checking, &transfers), 3000);
        assert_eq!(compute_balance(salary, &transfers), -5000);
        assert_eq!(compute_balance(groceries, &transfers), 2000);
    }

    #[test]
    fn test_compute_all_balances() {
        let checking = Uuid::new_v4();
        let salary = Uuid::new_v4();
        let groceries = Uuid::new_v4();

        let transfers = vec![
            make_transfer(salary, checking, 5000),
            make_transfer(checking, groceries, 2000),
        ];

        let balances = compute_all_balances(&transfers);

        assert_eq!(balances.get(&checking), Some(&3000));
        assert_eq!(balances.get(&salary), Some(&-5000));
        assert_eq!(balances.get(&groceries), Some(&2000));
    }

    #[test]
    fn test_balances_sum_to_zero() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let transfers = vec![
            make_transfer(a, b, 1000),
            make_transfer(b, c, 500),
            make_transfer(c, a, 200),
            make_transfer(a, c, 300),
        ];

        let balances = compute_all_balances(&transfers);
        let total: Cents = balances.values().sum();

        assert_eq!(total, 0, "All balances must sum to zero (closed system)");
    }

    #[test]
    fn test_validate_reversal_success() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let original = make_transfer(from, to, 10000);

        let result = validate_reversal(&original, 5000, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_reversal_exceeds_amount() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let original = make_transfer(from, to, 10000);

        let result = validate_reversal(&original, 15000, &[]);
        assert!(matches!(
            result,
            Err(ReversalError::ExceedsOriginalAmount { .. })
        ));
    }

    #[test]
    fn test_validate_reversal_with_existing_partial() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let original = make_transfer(from, to, 10000);

        // Existing partial reversal of 6000
        let partial = original.create_partial_reversal(6000);

        // Trying to reverse another 6000 should fail (total would be 12000 > 10000)
        let result = validate_reversal(&original, 6000, &[partial]);
        assert!(matches!(
            result,
            Err(ReversalError::ExceedsOriginalAmount { .. })
        ));
    }
}
