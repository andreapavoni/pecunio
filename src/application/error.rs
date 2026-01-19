use thiserror::Error;

use crate::domain::{Cents, WalletId};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Wallet already exists: {0}")]
    WalletAlreadyExists(String),

    #[error("Transfer not found: {0}")]
    TransferNotFound(String),

    #[error("Insufficient funds in wallet {wallet_name}: balance {balance}, required {required}")]
    InsufficientFunds {
        wallet_name: String,
        balance: Cents,
        required: Cents,
    },

    #[error("Currency mismatch between wallets: {from_currency} vs {to_currency}")]
    CurrencyMismatch {
        from_currency: String,
        to_currency: String,
    },

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Wallet is archived: {0}")]
    WalletArchived(String),

    #[error("Cannot reverse more than original amount")]
    ReversalExceedsOriginal {
        original_id: WalletId,
        original_amount: Cents,
        already_reversed: Cents,
        requested: Cents,
    },

    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
}
