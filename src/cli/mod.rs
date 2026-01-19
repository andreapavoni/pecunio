use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};

use crate::domain::{format_cents, parse_cents, Transfer, Wallet, WalletType};
use crate::storage::Repository;

/// Pecunio - Personal Finance Ledger
#[derive(Parser)]
#[command(name = "pecunio")]
#[command(about = "A local-first personal finance tool based on a transfer-based ledger")]
#[command(version)]
pub struct Cli {
    /// Database file path
    #[arg(short, long, default_value = "pecunio.db")]
    pub database: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new database
    Init,

    /// Wallet management commands
    #[command(subcommand)]
    Wallet(WalletCommands),

    /// Record a transfer between wallets
    Transfer {
        /// Amount to transfer (e.g., "50.00" or "50")
        amount: String,

        /// Source wallet name
        #[arg(long)]
        from: String,

        /// Destination wallet name
        #[arg(long)]
        to: String,

        /// Description of the transfer
        #[arg(short, long)]
        description: Option<String>,

        /// Category for budgeting (e.g., "groceries", "utilities")
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Show balance for a wallet or all wallets
    Balance {
        /// Wallet name (omit for all wallets)
        wallet: Option<String>,
    },

    /// List recent transfers
    Transfers {
        /// Filter by wallet name
        #[arg(long)]
        wallet: Option<String>,

        /// Maximum number of transfers to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Create a new wallet
    Create {
        /// Wallet name (must be unique)
        name: String,

        /// Wallet type: asset, liability, income, expense, equity
        #[arg(short = 't', long = "type")]
        wallet_type: String,

        /// Currency code (e.g., EUR, USD)
        #[arg(short, long, default_value = "EUR")]
        currency: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List all wallets
    List {
        /// Include archived wallets
        #[arg(long)]
        all: bool,
    },

    /// Archive a wallet (soft delete)
    Archive {
        /// Wallet name
        name: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Init => {
                let db_url = format!("sqlite:{}?mode=rwc", self.database);
                Repository::init(&db_url).await?;
                println!("Database initialized: {}", self.database);
            }

            Commands::Wallet(wallet_cmd) => {
                let db_url = format!("sqlite:{}", self.database);
                let repo = Repository::connect(&db_url).await?;

                match wallet_cmd {
                    WalletCommands::Create {
                        name,
                        wallet_type,
                        currency,
                        description,
                    } => {
                        let wt = WalletType::from_str(&wallet_type).ok_or_else(|| {
                            anyhow::anyhow!(
                                "Invalid wallet type '{}'. Valid types: asset, liability, income, expense, equity",
                                wallet_type
                            )
                        })?;

                        let mut wallet = Wallet::new(name.clone(), wt, currency);
                        if let Some(desc) = description {
                            wallet = wallet.with_description(desc);
                        }

                        repo.save_wallet(&wallet).await?;
                        println!("Created wallet: {} ({})", name, wt);
                    }

                    WalletCommands::List { all } => {
                        let wallets = repo.list_wallets(all).await?;
                        if wallets.is_empty() {
                            println!("No wallets found.");
                        } else {
                            println!("{:<20} {:<12} {:<8}", "NAME", "TYPE", "CURRENCY");
                            println!("{}", "-".repeat(44));
                            for wallet in wallets {
                                println!(
                                    "{:<20} {:<12} {:<8}",
                                    wallet.name, wallet.wallet_type, wallet.currency
                                );
                            }
                        }
                    }

                    WalletCommands::Archive { name } => {
                        let wallet = repo
                            .get_wallet_by_name(&name)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("Wallet '{}' not found", name))?;
                        repo.archive_wallet(wallet.id).await?;
                        println!("Archived wallet: {}", name);
                    }
                }
            }

            Commands::Transfer {
                amount,
                from,
                to,
                description,
                category,
            } => {
                let db_url = format!("sqlite:{}", self.database);
                let repo = Repository::connect(&db_url).await?;

                let amount_cents =
                    parse_cents(&amount).context("Invalid amount format. Use '50.00' or '50'")?;

                let from_wallet = repo
                    .get_wallet_by_name(&from)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Source wallet '{}' not found", from))?;

                let to_wallet = repo
                    .get_wallet_by_name(&to)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Destination wallet '{}' not found", to))?;

                // Validate currencies match
                if from_wallet.currency != to_wallet.currency {
                    bail!(
                        "Currency mismatch: {} ({}) vs {} ({})",
                        from,
                        from_wallet.currency,
                        to,
                        to_wallet.currency
                    );
                }

                let mut transfer =
                    Transfer::new(from_wallet.id, to_wallet.id, amount_cents, Utc::now());

                if let Some(desc) = description {
                    transfer = transfer.with_description(desc);
                }
                if let Some(cat) = category {
                    transfer = transfer.with_category(cat);
                }

                repo.save_transfer(&mut transfer).await?;
                println!(
                    "Recorded transfer: {} {} -> {} ({})",
                    format_cents(amount_cents),
                    from,
                    to,
                    transfer.id
                );
            }

            Commands::Balance { wallet } => {
                let db_url = format!("sqlite:{}", self.database);
                let repo = Repository::connect(&db_url).await?;

                match wallet {
                    Some(name) => {
                        let w = repo
                            .get_wallet_by_name(&name)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("Wallet '{}' not found", name))?;
                        let balance = repo.compute_balance(w.id).await?;
                        println!("{}: {} {}", name, format_cents(balance), w.currency);
                    }
                    None => {
                        let wallets = repo.list_wallets(false).await?;
                        if wallets.is_empty() {
                            println!("No wallets found.");
                        } else {
                            println!("{:<20} {:>12} {:<8}", "WALLET", "BALANCE", "CURRENCY");
                            println!("{}", "-".repeat(44));
                            for wallet in wallets {
                                let balance = repo.compute_balance(wallet.id).await?;
                                println!(
                                    "{:<20} {:>12} {:<8}",
                                    wallet.name,
                                    format_cents(balance),
                                    wallet.currency
                                );
                            }
                        }
                    }
                }
            }

            Commands::Transfers { wallet, limit } => {
                let db_url = format!("sqlite:{}", self.database);
                let repo = Repository::connect(&db_url).await?;

                let transfers = match wallet {
                    Some(name) => {
                        let w = repo
                            .get_wallet_by_name(&name)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("Wallet '{}' not found", name))?;
                        repo.list_transfers_for_wallet(w.id).await?
                    }
                    None => repo.list_transfers().await?,
                };

                if transfers.is_empty() {
                    println!("No transfers found.");
                } else {
                    // Get wallet names for display
                    let wallets = repo.list_wallets(true).await?;
                    let wallet_names: std::collections::HashMap<_, _> =
                        wallets.iter().map(|w| (w.id, w.name.as_str())).collect();

                    println!(
                        "{:<12} {:>10} {:<15} {:<15} {}",
                        "DATE", "AMOUNT", "FROM", "TO", "DESCRIPTION"
                    );
                    println!("{}", "-".repeat(70));

                    for transfer in transfers.iter().rev().take(limit) {
                        let from_name = wallet_names.get(&transfer.from_wallet).unwrap_or(&"?");
                        let to_name = wallet_names.get(&transfer.to_wallet).unwrap_or(&"?");
                        let date = transfer.timestamp.format("%Y-%m-%d");
                        let desc = transfer.description.as_deref().unwrap_or("");

                        println!(
                            "{:<12} {:>10} {:<15} {:<15} {}",
                            date,
                            format_cents(transfer.amount_cents),
                            truncate(from_name, 15),
                            truncate(to_name, 15),
                            truncate(desc, 30)
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
