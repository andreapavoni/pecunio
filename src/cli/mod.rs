use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::application::LedgerService;
use crate::domain::{format_cents, parse_cents, WalletType};

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

        /// Force transfer even if it would make wallet balance negative
        #[arg(long)]
        force: bool,
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

    /// Verify ledger integrity
    Check,

    /// Reverse a transfer (full or partial)
    Reverse {
        /// Transfer ID to reverse
        id: String,

        /// Amount to reverse (omit for full reversal)
        #[arg(short, long)]
        amount: Option<String>,
    },

    /// Show detailed transfer information
    #[command(name = "show")]
    ShowTransfer {
        /// Transfer ID
        id: String,
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

    /// Show detailed wallet information
    Show {
        /// Wallet name
        name: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Init => {
                LedgerService::init(&self.database).await?;
                println!("Database initialized: {}", self.database);
            }

            Commands::Wallet(wallet_cmd) => {
                let service = LedgerService::connect(&self.database).await?;
                run_wallet_command(&service, wallet_cmd).await?;
            }

            Commands::Transfer {
                amount,
                from,
                to,
                description,
                category,
                force,
            } => {
                let service = LedgerService::connect(&self.database).await?;
                let amount_cents =
                    parse_cents(&amount).context("Invalid amount format. Use '50.00' or '50'")?;

                let result = service
                    .record_transfer(&from, &to, amount_cents, description, category, force)
                    .await?;

                println!(
                    "Recorded transfer: {} {} -> {} ({})",
                    format_cents(result.transfer.amount_cents),
                    result.from_wallet_name,
                    result.to_wallet_name,
                    result.transfer.id
                );
            }

            Commands::Balance { wallet } => {
                let service = LedgerService::connect(&self.database).await?;
                run_balance_command(&service, wallet).await?;
            }

            Commands::Transfers { wallet, limit } => {
                let service = LedgerService::connect(&self.database).await?;
                run_transfers_command(&service, wallet, limit).await?;
            }

            Commands::Check => {
                let service = LedgerService::connect(&self.database).await?;
                run_check_command(&service).await?;
            }

            Commands::Reverse { id, amount } => {
                let service = LedgerService::connect(&self.database).await?;
                let transfer_id =
                    Uuid::parse_str(&id).context("Invalid transfer ID format (expected UUID)")?;

                let amount_cents = amount
                    .map(|a| parse_cents(&a))
                    .transpose()
                    .context("Invalid amount format for partial reversal")?;

                let result = service.reverse_transfer(transfer_id, amount_cents).await?;

                if result.is_partial {
                    println!(
                        "Partially reversed: {} of {}",
                        format_cents(result.reversal.amount_cents),
                        format_cents(result.original.amount_cents)
                    );
                } else {
                    println!(
                        "Reversed transfer: {} {} -> {}",
                        format_cents(result.original.amount_cents),
                        result.from_wallet_name,
                        result.to_wallet_name
                    );
                }
                println!(
                    "Created reversal: {} {} -> {} ({})",
                    format_cents(result.reversal.amount_cents),
                    result.to_wallet_name,
                    result.from_wallet_name,
                    result.reversal.id
                );
            }

            Commands::ShowTransfer { id } => {
                let service = LedgerService::connect(&self.database).await?;
                let transfer_id =
                    Uuid::parse_str(&id).context("Invalid transfer ID format (expected UUID)")?;

                run_show_transfer_command(&service, transfer_id).await?;
            }
        }

        Ok(())
    }
}

async fn run_wallet_command(service: &LedgerService, cmd: WalletCommands) -> Result<()> {
    match cmd {
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

            let wallet = service
                .create_wallet(name.clone(), wt, currency, description)
                .await?;
            println!("Created wallet: {} ({})", wallet.name, wallet.wallet_type);
        }

        WalletCommands::List { all } => {
            let wallets = service.list_wallets(all).await?;
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
            service.archive_wallet(&name).await?;
            println!("Archived wallet: {}", name);
        }

        WalletCommands::Show { name } => {
            let info = service.get_wallet_info(&name).await?;
            let wallet = &info.wallet;

            println!("Wallet: {}", wallet.name);
            println!("  ID:             {}", wallet.id);
            println!("  Type:           {}", wallet.wallet_type);
            println!("  Currency:       {}", wallet.currency);
            println!(
                "  Allow negative: {}",
                if wallet.allow_negative { "yes" } else { "no" }
            );
            if let Some(desc) = &wallet.description {
                println!("  Description:    {}", desc);
            }
            println!(
                "  Created:        {}",
                wallet.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            if let Some(archived) = wallet.archived_at {
                println!("  Archived:       {}", archived.format("%Y-%m-%d %H:%M:%S"));
            }
            println!();
            println!(
                "  Balance:        {} {}",
                format_cents(info.balance),
                wallet.currency
            );
            println!(
                "  Transfers:      {} ({} in, {} out)",
                info.incoming_count + info.outgoing_count,
                info.incoming_count,
                info.outgoing_count
            );
            if let Some(last) = info.last_activity {
                println!("  Last activity:  {}", last.format("%Y-%m-%d %H:%M:%S"));
            }
        }
    }
    Ok(())
}

async fn run_balance_command(service: &LedgerService, wallet: Option<String>) -> Result<()> {
    match wallet {
        Some(name) => {
            let entry = service.get_balance(&name).await?;
            println!(
                "{}: {} {}",
                entry.wallet.name,
                format_cents(entry.balance),
                entry.wallet.currency
            );
        }
        None => {
            let entries = service.get_all_balances().await?;
            if entries.is_empty() {
                println!("No wallets found.");
            } else {
                println!("{:<20} {:>12} {:<8}", "WALLET", "BALANCE", "CURRENCY");
                println!("{}", "-".repeat(44));
                for entry in entries {
                    println!(
                        "{:<20} {:>12} {:<8}",
                        entry.wallet.name,
                        format_cents(entry.balance),
                        entry.wallet.currency
                    );
                }
            }
        }
    }
    Ok(())
}

async fn run_transfers_command(
    service: &LedgerService,
    wallet: Option<String>,
    limit: usize,
) -> Result<()> {
    let transfers = service.list_transfers(wallet.as_deref()).await?;

    if transfers.is_empty() {
        println!("No transfers found.");
    } else {
        let wallet_names = service.get_wallet_names().await?;

        println!(
            "{:<12} {:>10} {:<15} {:<15} {}",
            "DATE", "AMOUNT", "FROM", "TO", "DESCRIPTION"
        );
        println!("{}", "-".repeat(70));

        for transfer in transfers.iter().rev().take(limit) {
            let from_name = wallet_names
                .get(&transfer.from_wallet)
                .map(|s| s.as_str())
                .unwrap_or("?");
            let to_name = wallet_names
                .get(&transfer.to_wallet)
                .map(|s| s.as_str())
                .unwrap_or("?");
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
    Ok(())
}

async fn run_check_command(service: &LedgerService) -> Result<()> {
    println!("Checking ledger integrity...\n");

    let report = service.check_integrity().await?;

    println!("Wallets:   {}", report.wallet_count);
    println!("Transfers: {}", report.transfer_count);
    println!();

    println!("Balance by type:");
    for wt in [
        WalletType::Asset,
        WalletType::Liability,
        WalletType::Income,
        WalletType::Expense,
        WalletType::Equity,
    ] {
        let balance = report.balance_by_type.get(&wt).copied().unwrap_or(0);
        println!("  {:<12} {:>12}", format!("{}:", wt), format_cents(balance));
    }
    println!("  {}", "-".repeat(26));
    println!(
        "  {:<12} {:>12}  {}",
        "Total:",
        format_cents(report.total_balance),
        if report.is_balanced {
            "OK"
        } else {
            "UNBALANCED!"
        }
    );
    println!();

    if report.is_healthy() {
        println!("Ledger is consistent.");
    } else {
        println!("Issues found:");
        for issue in &report.issues {
            println!("  - {}", issue);
        }
        anyhow::bail!("Ledger integrity check failed");
    }

    Ok(())
}

async fn run_show_transfer_command(service: &LedgerService, transfer_id: uuid::Uuid) -> Result<()> {
    let info = service.get_transfer_info(transfer_id).await?;
    let transfer = &info.transfer;

    println!("Transfer: {}", transfer.id);
    println!("  Sequence:    {}", transfer.sequence);
    println!(
        "  Date:        {}",
        transfer.timestamp.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  Amount:      {} {}",
        format_cents(transfer.amount_cents),
        info.from_wallet.currency
    );
    println!("  From:        {}", info.from_wallet.name);
    println!("  To:          {}", info.to_wallet.name);
    if let Some(cat) = &transfer.category {
        println!("  Category:    {}", cat);
    }
    if let Some(desc) = &transfer.description {
        println!("  Description: {}", desc);
    }
    if !transfer.tags.is_empty() {
        println!("  Tags:        {}", transfer.tags.join(", "));
    }
    if let Some(ext_ref) = &transfer.external_ref {
        println!("  External ref: {}", ext_ref);
    }
    println!(
        "  Recorded at: {}",
        transfer.recorded_at.format("%Y-%m-%d %H:%M:%S")
    );

    // Show reversal info
    if let Some(reverses_id) = transfer.reverses {
        println!();
        println!("  This is a reversal of: {}", reverses_id);
    }

    if !info.reversals.is_empty() {
        let remaining = transfer.amount_cents - info.total_reversed;
        let percentage = (info.total_reversed as f64 / transfer.amount_cents as f64) * 100.0;

        println!();
        println!("  Reversal status:");
        println!(
            "    Reversed:  {} ({:.0}%)",
            format_cents(info.total_reversed),
            percentage
        );
        println!("    Remaining: {}", format_cents(remaining));
        println!("    Reversals:");
        for rev in &info.reversals {
            println!(
                "      - {} on {} ({})",
                format_cents(rev.amount_cents),
                rev.timestamp.format("%Y-%m-%d"),
                rev.id
            );
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
