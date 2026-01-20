use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
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

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

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

        /// Date of the transfer (ISO 8601 format: YYYY-MM-DD, defaults to now)
        #[arg(long)]
        date: Option<String>,
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

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Filter from date (YYYY-MM-DD)
        #[arg(long)]
        from_date: Option<String>,

        /// Filter to date (YYYY-MM-DD)
        #[arg(long)]
        to_date: Option<String>,

        /// Maximum number of transfers to show
        #[arg(short, long)]
        limit: Option<usize>,
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

    /// Budget management commands
    #[command(subcommand)]
    Budget(BudgetCommands),

    /// Scheduled transfer management commands
    #[command(subcommand)]
    Scheduled(ScheduledCommands),

    /// Forecast future balances based on scheduled transfers
    Forecast {
        /// Number of months to forecast
        #[arg(short, long, default_value = "3")]
        months: usize,

        /// Filter by specific wallet (omit for all wallets)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Generate reports and analytics
    #[command(subcommand)]
    Report(ReportCommands),

    /// Export data to CSV or JSON
    Export {
        /// What to export: transfers, balances, budgets, scheduled, full
        export_type: String,

        /// Output file (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,

        /// Format: csv, json (default: csv for most types, json for full)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Import data from CSV or JSON
    Import {
        /// What to import: transfers, full
        import_type: String,

        /// Input file (stdin if omitted)
        #[arg(short, long)]
        input: Option<String>,

        /// Preview without importing
        #[arg(long)]
        dry_run: bool,

        /// Skip duplicate records
        #[arg(long)]
        skip_duplicates: bool,

        /// Create wallets that don't exist
        #[arg(long)]
        create_wallets: bool,

        /// Validate without importing
        #[arg(long)]
        validate: bool,
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

#[derive(Subcommand)]
pub enum BudgetCommands {
    /// Create a new budget
    Create {
        /// Budget name (must be unique)
        name: String,

        /// Category to track
        #[arg(short, long)]
        category: String,

        /// Budget amount (e.g., "400" or "400.00")
        #[arg(short, long)]
        amount: String,

        /// Period: weekly, monthly, yearly
        #[arg(short, long)]
        period: String,
    },

    /// List all budgets
    List,

    /// Show budget status (spending vs limit)
    Status,

    /// Delete a budget
    Delete {
        /// Budget name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ScheduledCommands {
    /// Create a new scheduled transfer
    Create {
        /// Scheduled transfer name (must be unique)
        name: String,

        /// Source wallet name
        #[arg(long)]
        from: String,

        /// Destination wallet name
        #[arg(long)]
        to: String,

        /// Amount to transfer (e.g., "50.00" or "50")
        #[arg(short, long)]
        amount: String,

        /// Recurrence pattern: daily, weekly, monthly, yearly
        #[arg(short, long)]
        pattern: String,

        /// Start date (ISO 8601 format: YYYY-MM-DD)
        #[arg(long)]
        start_date: String,

        /// Optional end date (ISO 8601 format: YYYY-MM-DD)
        #[arg(long)]
        end_date: Option<String>,

        /// Description of the transfer
        #[arg(short, long)]
        description: Option<String>,

        /// Category for budgeting
        #[arg(short, long)]
        category: Option<String>,
    },

    /// List all scheduled transfers
    List {
        /// Include paused and completed schedules
        #[arg(long)]
        all: bool,
    },

    /// Show detailed information about a scheduled transfer
    Show {
        /// Scheduled transfer name
        name: String,
    },

    /// Pause a scheduled transfer
    Pause {
        /// Scheduled transfer name
        name: String,
    },

    /// Resume a paused scheduled transfer
    Resume {
        /// Scheduled transfer name
        name: String,
    },

    /// Delete a scheduled transfer
    Delete {
        /// Scheduled transfer name
        name: String,
    },

    /// Execute all due scheduled transfers
    Execute {
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Execute a specific scheduled transfer
    Run {
        /// Scheduled transfer name
        name: String,

        /// Specific date to execute (ISO 8601 format: YYYY-MM-DD, defaults to next due date)
        #[arg(long)]
        date: Option<String>,

        /// Force execution even if not due
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ReportCommands {
    /// Category spending breakdown
    Spending {
        /// Start date (YYYY-MM-DD, defaults to start of current month)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD, defaults to now)
        #[arg(long)]
        to: Option<String>,

        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Income vs Expense analysis
    IncomeExpense {
        /// Start date (YYYY-MM-DD, defaults to start of current month)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD, defaults to now)
        #[arg(long)]
        to: Option<String>,

        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Cash flow over time
    Cashflow {
        /// Start date (YYYY-MM-DD, defaults to start of current month)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD, defaults to now)
        #[arg(long)]
        to: Option<String>,

        /// Period: weekly, monthly, yearly
        #[arg(long, default_value = "monthly")]
        period: String,

        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Net worth summary
    NetWorth {
        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Compare current period to previous
    Compare {
        /// Period: weekly, monthly, yearly
        #[arg(long, default_value = "monthly")]
        period: String,

        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },
}

impl Cli {
    async fn auto_execute_scheduled(&self, service: &LedgerService) -> Result<()> {
        let now = Utc::now();
        let results = service.execute_due_scheduled_transfers(now).await?;

        // Log only if verbose flag is set
        if self.verbose && !results.is_empty() {
            eprintln!(
                "[Auto-exec] Executed {} scheduled transfer(s)",
                results.len()
            );
            for result in results {
                eprintln!(
                    "  {} -> {}: {}",
                    result.from_wallet_name,
                    result.to_wallet_name,
                    format_cents(result.transfer.amount_cents)
                );
            }
        }
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        // Auto-execute scheduled transfers before command dispatch (except for Init)
        if !matches!(self.command, Commands::Init) {
            if let Ok(service) = LedgerService::connect(&self.database).await {
                let _ = self.auto_execute_scheduled(&service).await;
            }
        }

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
                date,
            } => {
                let service = LedgerService::connect(&self.database).await?;
                let amount_cents =
                    parse_cents(&amount).context("Invalid amount format. Use '50.00' or '50'")?;

                // Parse date or use now
                let timestamp = match date {
                    Some(date_str) => parse_date(&date_str).with_context(|| {
                        format!("Invalid date format '{}'. Use YYYY-MM-DD", date_str)
                    })?,
                    None => Utc::now(),
                };

                let result = service
                    .record_transfer(
                        &from,
                        &to,
                        amount_cents,
                        timestamp,
                        description,
                        category,
                        force,
                    )
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

            Commands::Transfers {
                wallet,
                category,
                from_date,
                to_date,
                limit,
            } => {
                let service = LedgerService::connect(&self.database).await?;
                run_transfers_command(&service, wallet, category, from_date, to_date, limit)
                    .await?;
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

            Commands::Budget(budget_cmd) => {
                let service = LedgerService::connect(&self.database).await?;
                run_budget_command(&service, budget_cmd).await?;
            }

            Commands::Scheduled(scheduled_cmd) => {
                let service = LedgerService::connect(&self.database).await?;
                run_scheduled_command(&service, scheduled_cmd).await?;
            }

            Commands::Forecast { months, wallet } => {
                let service = LedgerService::connect(&self.database).await?;
                run_forecast_command(&service, months, wallet.as_deref()).await?;
            }

            Commands::Report(report_cmd) => {
                let service = LedgerService::connect(&self.database).await?;
                run_report_command(&service, report_cmd).await?;
            }

            Commands::Export {
                export_type,
                output,
                format,
            } => {
                let service = LedgerService::connect(&self.database).await?;
                run_export_command(&service, &export_type, output.as_deref(), format.as_deref())
                    .await?;
            }

            Commands::Import {
                import_type,
                input,
                dry_run,
                skip_duplicates,
                create_wallets,
                validate,
            } => {
                let service = LedgerService::connect(&self.database).await?;
                run_import_command(
                    &service,
                    &import_type,
                    input.as_deref(),
                    dry_run,
                    skip_duplicates,
                    create_wallets,
                    validate,
                )
                .await?;
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
            let wt: WalletType = wallet_type.parse().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid wallet type '{}'. Valid types: asset, liability, income, expense, equity. Error: {}",
                    wallet_type,
                    e
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

async fn run_export_command(
    service: &LedgerService,
    export_type: &str,
    output: Option<&str>,
    _format: Option<&str>,
) -> Result<()> {
    use crate::io::Exporter;
    use std::fs::File;
    use std::io::{stdout, Write};

    let exporter = Exporter::new(service);

    // Determine output writer
    let writer: Box<dyn Write> = match output {
        Some(path) => {
            let file = File::create(path)
                .with_context(|| format!("Failed to create output file: {}", path))?;
            Box::new(file)
        }
        None => Box::new(stdout()),
    };

    match export_type {
        "transfers" => {
            let count = exporter.export_transfers_csv(writer).await?;
            if output.is_some() {
                eprintln!("Exported {} transfers", count);
            }
        }
        "balances" => {
            let count = exporter.export_balances_csv(writer).await?;
            if output.is_some() {
                eprintln!("Exported {} balances", count);
            }
        }
        "budgets" => {
            let count = exporter.export_budgets_csv(writer).await?;
            if output.is_some() {
                eprintln!("Exported {} budgets", count);
            }
        }
        "scheduled" => {
            let count = exporter.export_scheduled_csv(writer).await?;
            if output.is_some() {
                eprintln!("Exported {} scheduled transfers", count);
            }
        }
        "full" => {
            let snapshot = exporter.export_full_json(writer).await?;
            if output.is_some() {
                eprintln!(
                    "Exported full database: {} wallets, {} transfers, {} budgets, {} scheduled transfers",
                    snapshot.wallets.len(),
                    snapshot.transfers.len(),
                    snapshot.budgets.len(),
                    snapshot.scheduled_transfers.len()
                );
            }
        }
        _ => {
            anyhow::bail!(
                "Invalid export type '{}'. Valid types: transfers, balances, budgets, scheduled, full",
                export_type
            );
        }
    }

    Ok(())
}

async fn run_import_command(
    service: &LedgerService,
    import_type: &str,
    input: Option<&str>,
    dry_run: bool,
    skip_duplicates: bool,
    create_wallets: bool,
    validate: bool,
) -> Result<()> {
    use crate::io::{ImportOptions, Importer};
    use std::fs::File;
    use std::io::{stdin, Read};

    let importer = Importer::new(service);

    // Determine input reader
    let reader: Box<dyn Read> = match input {
        Some(path) => {
            let file =
                File::open(path).with_context(|| format!("Failed to open input file: {}", path))?;
            Box::new(file)
        }
        None => Box::new(stdin()),
    };

    let options = ImportOptions {
        dry_run,
        skip_duplicates,
        create_missing_wallets: create_wallets,
        validate_only: validate,
    };

    let result = match import_type {
        "transfers" => importer.import_transfers_csv(reader, options).await?,
        "full" => importer.import_full_json(reader, options).await?,
        _ => {
            anyhow::bail!(
                "Invalid import type '{}'. Valid types: transfers, full",
                import_type
            );
        }
    };

    // Display results
    if validate || dry_run {
        println!("Validation successful");
    } else {
        println!("Import complete");
    }
    println!("  Imported: {}", result.imported);
    println!("  Skipped:  {}", result.skipped);
    println!("  Errors:   {}", result.errors.len());

    if !result.errors.is_empty() {
        println!("\nErrors:");
        for error in result.errors.iter().take(10) {
            println!(
                "  Line {}: {}",
                error.line,
                error
                    .field
                    .as_ref()
                    .map(|f| format!("{}: ", f))
                    .unwrap_or_default()
                    + &error.error
            );
        }
        if result.errors.len() > 10 {
            println!("  ... and {} more errors", result.errors.len() - 10);
        }
    }

    Ok(())
}

async fn run_report_command(service: &LedgerService, cmd: ReportCommands) -> Result<()> {
    use crate::domain::PeriodType;

    match cmd {
        ReportCommands::Spending { from, to, format } => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let report = service.get_category_report(from_date, to_date).await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "csv" => {
                    println!("category,total,count,average,percentage");
                    for cat in &report.categories {
                        println!(
                            "{},{},{},{},{:.2}",
                            cat.category, cat.total, cat.count, cat.average, cat.percentage
                        );
                    }
                }
                _ => {
                    // Table format
                    println!("Category Spending Report");
                    println!(
                        "Period: {} to {}",
                        from_date.format("%Y-%m-%d"),
                        to_date.format("%Y-%m-%d")
                    );
                    println!();
                    println!(
                        "{:<20} {:>12} {:>8} {:>12} {:>8}",
                        "CATEGORY", "TOTAL", "COUNT", "AVERAGE", "PERCENT"
                    );
                    println!("{}", "-".repeat(65));

                    for cat in &report.categories {
                        println!(
                            "{:<20} {:>12} {:>8} {:>12} {:>7.1}%",
                            truncate(&cat.category, 20),
                            format_cents(cat.total),
                            cat.count,
                            format_cents(cat.average),
                            cat.percentage
                        );
                    }

                    println!("{}", "-".repeat(65));
                    println!("{:<20} {:>12}", "TOTAL", format_cents(report.total));
                }
            }
        }

        ReportCommands::IncomeExpense { from, to, format } => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let report = service
                .get_income_expense_report(from_date, to_date)
                .await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "csv" => {
                    println!("type,amount");
                    println!("income,{}", report.total_income);
                    println!("expense,{}", report.total_expense);
                    println!("net,{}", report.net);
                }
                _ => {
                    // Table format
                    println!("Income vs Expense Report");
                    println!(
                        "Period: {} to {}",
                        from_date.format("%Y-%m-%d"),
                        to_date.format("%Y-%m-%d")
                    );
                    println!();
                    println!("Total Income:   {:>15}", format_cents(report.total_income));
                    println!("Total Expense:  {:>15}", format_cents(report.total_expense));
                    println!("{}", "-".repeat(32));
                    println!("Net:            {:>15}", format_cents(report.net));

                    if !report.expense_categories.is_empty() {
                        println!();
                        println!("Top Expense Categories:");
                        for (i, cat) in report.expense_categories.iter().take(10).enumerate() {
                            println!(
                                "  {}. {:<18} {:>12} ({:.1}%)",
                                i + 1,
                                truncate(&cat.category, 18),
                                format_cents(cat.total),
                                cat.percentage
                            );
                        }
                    }
                }
            }
        }

        ReportCommands::Cashflow {
            from,
            to,
            period,
            format,
        } => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let period_type: PeriodType = period.parse().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid period '{}'. Valid: weekly, monthly, yearly. Error: {}",
                    period,
                    e
                )
            })?;

            let report = service
                .get_cashflow_report(from_date, to_date, period_type)
                .await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "csv" => {
                    println!("period_start,period_end,inflow,outflow,net");
                    for period in &report.periods {
                        println!(
                            "{},{},{},{},{}",
                            period.period_start.format("%Y-%m-%d"),
                            period.period_end.format("%Y-%m-%d"),
                            period.inflow,
                            period.outflow,
                            period.net
                        );
                    }
                }
                _ => {
                    // Table format
                    println!("Cash Flow Report");
                    println!(
                        "Period: {} to {}",
                        from_date.format("%Y-%m-%d"),
                        to_date.format("%Y-%m-%d")
                    );
                    println!();
                    println!(
                        "{:<12} {:>12} {:>12} {:>12}",
                        "PERIOD", "INFLOW", "OUTFLOW", "NET"
                    );
                    println!("{}", "-".repeat(52));

                    for period in &report.periods {
                        let period_label = period.period_start.format("%Y-%m-%d").to_string();
                        println!(
                            "{:<12} {:>12} {:>12} {:>12}",
                            truncate(&period_label, 12),
                            format_cents(period.inflow),
                            format_cents(period.outflow),
                            format_cents(period.net)
                        );
                    }
                }
            }
        }

        ReportCommands::NetWorth { format } => {
            let report = service.get_net_worth_report().await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "csv" => {
                    println!("type,wallet,balance");
                    for asset in &report.assets {
                        println!("asset,{},{}", asset.wallet_name, asset.balance);
                    }
                    for liability in &report.liabilities {
                        println!("liability,{},{}", liability.wallet_name, liability.balance);
                    }
                }
                _ => {
                    // Table format
                    println!("Net Worth Report");
                    println!("As of: {}", report.as_of.format("%Y-%m-%d %H:%M:%S"));
                    println!();

                    if !report.assets.is_empty() {
                        println!("Assets:");
                        for asset in &report.assets {
                            println!(
                                "  {:<25} {:>15}",
                                truncate(&asset.wallet_name, 25),
                                format_cents(asset.balance)
                            );
                        }
                        println!("  {:<25} {:>15}", "", "-".repeat(15));
                        println!(
                            "  {:<25} {:>15}",
                            "Total Assets",
                            format_cents(report.total_assets)
                        );
                        println!();
                    }

                    if !report.liabilities.is_empty() {
                        println!("Liabilities:");
                        for liability in &report.liabilities {
                            println!(
                                "  {:<25} {:>15}",
                                truncate(&liability.wallet_name, 25),
                                format_cents(liability.balance)
                            );
                        }
                        println!("  {:<25} {:>15}", "", "-".repeat(15));
                        println!(
                            "  {:<25} {:>15}",
                            "Total Liabilities",
                            format_cents(report.total_liabilities)
                        );
                        println!();
                    }

                    println!("{}", "=".repeat(44));
                    println!("{:<25} {:>15}", "Net Worth", format_cents(report.net_worth));
                }
            }
        }

        ReportCommands::Compare { period, format } => {
            let period_type: PeriodType = period.parse().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid period '{}'. Valid: weekly, monthly, yearly. Error: {}",
                    period,
                    e
                )
            })?;

            let report = service.get_period_comparison(period_type).await?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "csv" => {
                    println!("period,income,expense,net");
                    println!(
                        "current,{},{},{}",
                        report.current_period.total_income,
                        report.current_period.total_expense,
                        report.current_period.net
                    );
                    println!(
                        "previous,{},{},{}",
                        report.previous_period.total_income,
                        report.previous_period.total_expense,
                        report.previous_period.net
                    );
                }
                _ => {
                    // Table format
                    println!("Period Comparison Report");
                    println!();

                    println!(
                        "Current Period: {} to {}",
                        report.current_period.period_start.format("%Y-%m-%d"),
                        report.current_period.period_end.format("%Y-%m-%d")
                    );
                    println!(
                        "  Income:  {:>15}",
                        format_cents(report.current_period.total_income)
                    );
                    println!(
                        "  Expense: {:>15}",
                        format_cents(report.current_period.total_expense)
                    );
                    println!("  Net:     {:>15}", format_cents(report.current_period.net));
                    println!();

                    println!(
                        "Previous Period: {} to {}",
                        report.previous_period.period_start.format("%Y-%m-%d"),
                        report.previous_period.period_end.format("%Y-%m-%d")
                    );
                    println!(
                        "  Income:  {:>15}",
                        format_cents(report.previous_period.total_income)
                    );
                    println!(
                        "  Expense: {:>15}",
                        format_cents(report.previous_period.total_expense)
                    );
                    println!(
                        "  Net:     {:>15}",
                        format_cents(report.previous_period.net)
                    );
                    println!();

                    println!("{}", "=".repeat(44));
                    println!(
                        "Change:  {:>15} ({:+.1}%)",
                        format_cents(report.change),
                        report.change_percentage
                    );
                }
            }
        }
    }

    Ok(())
}

fn parse_date_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    use chrono::Datelike;

    let now = Utc::now();

    // Default to_date is now
    let to_date = match to {
        Some(date_str) => parse_date(&date_str)?,
        None => now,
    };

    // Default from_date is start of current month
    let from_date = match from {
        Some(date_str) => parse_date(&date_str)?,
        None => now
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
    };

    Ok((from_date, to_date))
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
    category: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<usize>,
) -> Result<()> {
    use crate::application::TransferFilter;

    // Parse dates
    let from_date_parsed = from_date
        .map(|s| parse_date(&s))
        .transpose()
        .context("Invalid from-date")?;
    let to_date_parsed = to_date
        .map(|s| parse_date(&s))
        .transpose()
        .context("Invalid to-date")?;

    let filter = TransferFilter {
        wallet,
        category,
        from_date: from_date_parsed,
        to_date: to_date_parsed,
        limit,
    };

    let transfers = service.list_transfers_filtered(filter).await?;

    if transfers.is_empty() {
        println!("No transfers found.");
    } else {
        let wallet_names = service.get_wallet_names().await?;

        println!(
            "{:<12} {:>10} {:<15} {:<15} DESCRIPTION",
            "DATE", "AMOUNT", "FROM", "TO"
        );
        println!("{}", "-".repeat(70));

        // Show all transfers (limit already applied in query)
        for transfer in transfers.iter().rev() {
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

fn parse_date(date_str: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    use chrono::NaiveDate;

    // Parse YYYY-MM-DD format
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .context("Date must be in YYYY-MM-DD format")?;

    // Convert to UTC datetime at midnight
    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;

    Ok(chrono::DateTime::from_naive_utc_and_offset(
        naive_datetime,
        chrono::Utc,
    ))
}

async fn run_budget_command(service: &LedgerService, cmd: BudgetCommands) -> Result<()> {
    use crate::domain::PeriodType;

    match cmd {
        BudgetCommands::Create {
            name,
            category,
            amount,
            period,
        } => {
            let amount_cents =
                parse_cents(&amount).context("Invalid amount format. Use '400.00' or '400'")?;

            let period_type: PeriodType = period.parse().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid period type '{}'. Valid types: weekly, monthly, yearly. Error: {}",
                    period,
                    e
                )
            })?;

            let budget = service
                .create_budget(name.clone(), category, amount_cents, period_type)
                .await?;
            println!(
                "Created budget: {} ({}, {} per {})",
                budget.name,
                budget.category,
                format_cents(budget.amount_cents),
                budget.period_type
            );
        }

        BudgetCommands::List => {
            let budgets = service.list_budgets().await?;
            if budgets.is_empty() {
                println!("No budgets found.");
            } else {
                println!(
                    "{:<20} {:<15} {:>12} {:<10}",
                    "NAME", "CATEGORY", "AMOUNT", "PERIOD"
                );
                println!("{}", "-".repeat(60));
                for budget in budgets {
                    println!(
                        "{:<20} {:<15} {:>12} {:<10}",
                        budget.name,
                        budget.category,
                        format_cents(budget.amount_cents),
                        budget.period_type
                    );
                }
            }
        }

        BudgetCommands::Status => {
            let statuses = service.get_all_budget_statuses().await?;
            if statuses.is_empty() {
                println!("No budgets found.");
            } else {
                println!(
                    "{:<20} {:<10} {:>12} {:>12} {:>12}",
                    "BUDGET", "PERIOD", "LIMIT", "SPENT", "REMAINING"
                );
                println!("{}", "-".repeat(70));
                for status in statuses {
                    println!(
                        "{:<20} {:<10} {:>12} {:>12} {:>12}",
                        status.budget.name,
                        status.budget.period_type,
                        format_cents(status.budget.amount_cents),
                        format_cents(status.spent),
                        format_cents(status.remaining),
                    );
                }
            }
        }

        BudgetCommands::Delete { name } => {
            service.delete_budget(&name).await?;
            println!("Deleted budget: {}", name);
        }
    }

    Ok(())
}

async fn run_scheduled_command(service: &LedgerService, command: ScheduledCommands) -> Result<()> {
    use crate::domain::RecurrencePattern;

    match command {
        ScheduledCommands::Create {
            name,
            from,
            to,
            amount,
            pattern,
            start_date,
            end_date,
            description,
            category,
        } => {
            let amount_cents = parse_cents(&amount)?;
            let pattern: RecurrencePattern = pattern
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid pattern: {}. Error: {}", pattern, e))?;
            let start = parse_date(&start_date)?;
            let end = end_date.as_deref().map(parse_date).transpose()?;

            let scheduled = service
                .create_scheduled_transfer(
                    name.clone(),
                    &from,
                    &to,
                    amount_cents,
                    pattern,
                    start,
                    end,
                    description,
                    category,
                )
                .await?;

            println!("Created scheduled transfer: {}", scheduled.name);
            println!("  From: {}", from);
            println!("  To: {}", to);
            println!("  Amount: {}", format_cents(scheduled.amount_cents));
            println!("  Pattern: {}", scheduled.pattern);
            println!("  Start: {}", scheduled.start_date.format("%Y-%m-%d"));
            if let Some(end_date) = scheduled.end_date {
                println!("  End: {}", end_date.format("%Y-%m-%d"));
            }
        }

        ScheduledCommands::List { all } => {
            let scheduled = service.list_scheduled_transfers(all).await?;
            if scheduled.is_empty() {
                println!("No scheduled transfers found.");
            } else {
                println!(
                    "{:<20} {:<15} {:<15} {:>12} {:<10} {:<12}",
                    "NAME", "FROM", "TO", "AMOUNT", "PATTERN", "STATUS"
                );
                println!("{}", "-".repeat(90));
                for st in scheduled {
                    // Get wallet names - we'll need to look them up
                    // For now, just show the first part of IDs
                    println!(
                        "{:<20} {:<15} {:<15} {:>12} {:<10} {:<12}",
                        truncate(&st.name, 20),
                        format!("{:.8}", st.from_wallet),
                        format!("{:.8}", st.to_wallet),
                        format_cents(st.amount_cents),
                        st.pattern,
                        st.status,
                    );
                }
            }
        }

        ScheduledCommands::Show { name } => {
            let st = service.get_scheduled_transfer(&name).await?;
            let now = Utc::now();

            println!("Scheduled Transfer: {}", st.name);
            println!("  ID: {}", st.id);
            println!("  Status: {}", st.status);
            println!("  Pattern: {}", st.pattern);
            println!("  Amount: {}", format_cents(st.amount_cents));
            println!("  Start Date: {}", st.start_date.format("%Y-%m-%d"));
            if let Some(end_date) = st.end_date {
                println!("  End Date: {}", end_date.format("%Y-%m-%d"));
            }
            if let Some(last_exec) = st.last_executed_at {
                println!("  Last Executed: {}", last_exec.format("%Y-%m-%d"));
            }
            if let Some(next) = st.next_execution_date(now) {
                println!("  Next Due: {}", next.format("%Y-%m-%d"));
            }
            if let Some(desc) = &st.description {
                println!("  Description: {}", desc);
            }
            if let Some(cat) = &st.category {
                println!("  Category: {}", cat);
            }
        }

        ScheduledCommands::Pause { name } => {
            service.pause_scheduled_transfer(&name).await?;
            println!("Paused scheduled transfer: {}", name);
        }

        ScheduledCommands::Resume { name } => {
            service.resume_scheduled_transfer(&name).await?;
            println!("Resumed scheduled transfer: {}", name);
        }

        ScheduledCommands::Delete { name } => {
            service.delete_scheduled_transfer(&name).await?;
            println!("Deleted scheduled transfer: {}", name);
        }

        ScheduledCommands::Execute { dry_run } => {
            let now = Utc::now();
            if dry_run {
                println!("DRY RUN - No transfers will be executed");
                let scheduled = service.list_scheduled_transfers(false).await?;
                for st in scheduled {
                    let pending = st.pending_executions(now);
                    if !pending.is_empty() {
                        println!("\n{}: {} pending execution(s)", st.name, pending.len());
                        for date in pending {
                            println!("  - {}", date.format("%Y-%m-%d"));
                        }
                    }
                }
            } else {
                let results = service.execute_due_scheduled_transfers(now).await?;
                if results.is_empty() {
                    println!("No scheduled transfers due for execution.");
                } else {
                    println!("Executed {} scheduled transfer(s):", results.len());
                    for result in results {
                        println!(
                            "  {} -> {}: {}",
                            result.from_wallet_name,
                            result.to_wallet_name,
                            format_cents(result.transfer.amount_cents)
                        );
                    }
                }
            }
        }

        ScheduledCommands::Run { name, date, force } => {
            let exec_date = date.as_deref().map(parse_date).transpose()?;
            let result = service
                .execute_scheduled_transfer(&name, exec_date, force)
                .await?;

            println!("Executed scheduled transfer: {}", name);
            println!(
                "  {} -> {}: {}",
                result.from_wallet_name,
                result.to_wallet_name,
                format_cents(result.transfer.amount_cents)
            );
            println!("  Transfer ID: {}", result.transfer.id);
        }
    }

    Ok(())
}

async fn run_forecast_command(
    service: &LedgerService,
    months: usize,
    wallet_filter: Option<&str>,
) -> Result<()> {
    let forecast = service.forecast_balances(months).await?;

    if forecast.snapshots.is_empty() {
        println!("No forecast data available.");
        return Ok(());
    }

    println!(
        "Forecast: {} to {}",
        forecast.start_date.format("%Y-%m-%d"),
        forecast.end_date.format("%Y-%m-%d")
    );
    println!();

    // Get all wallet names from the first snapshot
    let mut wallet_names: Vec<String> = forecast.snapshots[0]
        .wallet_balances
        .keys()
        .cloned()
        .collect();

    // Filter by wallet if specified
    if let Some(filter) = wallet_filter {
        wallet_names.retain(|name| name == filter);
        if wallet_names.is_empty() {
            println!("No wallet found with name: {}", filter);
            return Ok(());
        }
    }

    wallet_names.sort();

    // Print header
    print!("{:<12}", "DATE");
    for wallet in &wallet_names {
        print!("{:>15}", truncate(wallet, 15));
    }
    println!("{:<40}", "  EVENT");
    println!("{}", "-".repeat(80 + wallet_names.len() * 15));

    // Print snapshots
    for snapshot in &forecast.snapshots {
        print!("{:<12}", snapshot.date.format("%Y-%m-%d"));

        for wallet in &wallet_names {
            let balance = snapshot.wallet_balances.get(wallet).copied().unwrap_or(0);
            print!("{:>15}", format_cents(balance));
        }

        if let Some(event) = &snapshot.event {
            print!(
                "  {} ({} -> {})",
                event.scheduled_name,
                truncate(&event.from_wallet, 10),
                truncate(&event.to_wallet, 10)
            );
        }

        println!();
    }

    Ok(())
}
