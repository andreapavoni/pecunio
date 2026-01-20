# Pecunio

> A local-first personal finance ledger for the command line

**Pecunio** is a simple, powerful, and private personal finance tool that runs entirely on your machine. No cloud accounts, no subscriptions, no data sharing—just you and your financial data.

Built on a **transfer-based ledger** model (money flows between wallets), Pecunio gives you complete control over tracking income, expenses, budgets, and scheduled transfers with automatic execution and comprehensive reporting.

## Features

### Core Functionality
- **Transfer-based Ledger** - Track money flowing between wallets (accounts)
- **5 Wallet Types** - Asset, Liability, Income, Expense, Equity
- **Categories & Tags** - Organize transfers for budgeting and reporting
- **Date Support** - Record historical transfers with custom timestamps
- **Reversals** - Full and partial transfer reversals with audit trail

### Budgeting & Planning
- **Budgets** - Set spending limits by category (weekly, monthly, yearly)
- **Budget Tracking** - Real-time spending vs. limits with remaining balance
- **Scheduled Transfers** - Recurring transfers (salary, rent, subscriptions)
- **Auto-Execution** - Scheduled transfers execute automatically on every CLI invocation
- **Forecasting** - Project future balances based on scheduled transfers

### Reporting & Analytics
- **Category Spending** - Breakdown with totals, averages, percentages
- **Income vs Expense** - Net analysis with category breakdown
- **Cash Flow** - Track inflow/outflow by period
- **Net Worth** - Assets - Liabilities with detailed breakdown
- **Period Comparison** - Compare current vs previous period

### Data Portability
- **Export** - Export transfers, balances, budgets to CSV or JSON
- **Import** - Import transfers from CSV with validation
- **Full Backup** - Complete database snapshot as JSON
- **Bank Integration Ready** - Import from bank CSV exports

### Technical
- **SQLite Storage** - Fast, reliable, single-file database
- **Local-First** - All data stays on your machine
- **Data Integrity** - Built-in integrity checks and validation
- **Well-Tested** - Comprehensive test suite (29+ integration tests)
- **Rust** - Fast, safe, and efficient

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/pecunio.git
cd pecunio

# Build and install
cargo install --path .

# Or just build
cargo build --release
# Binary will be in target/release/pecunio
```

### Initialize Your Ledger

```bash
# Create a new database
pecunio init

# Or specify a custom location
pecunio -d ~/finances/ledger.db init
```

## Usage Guide

### Basic Workflow

1. **Create Wallets** (accounts)
2. **Record Transfers** (money movements)
3. **Check Balances**
4. **Generate Reports**

### 1. Creating Wallets

```bash
# Create an asset wallet (checking account)
pecunio wallet create Checking --type asset --currency USD

# Create income and expense wallets
pecunio wallet create Salary --type income
pecunio wallet create Groceries --type expense
pecunio wallet create Rent --type expense

# Create a liability (credit card)
pecunio wallet create CreditCard --type liability

# List all wallets
pecunio wallet list
```

**Wallet Types:**
- **Asset** - Bank accounts, cash, investments (things you own)
- **Liability** - Credit cards, loans (debts you owe)
- **Income** - Sources of money (salary, freelance, gifts)
- **Expense** - Destinations for spending (groceries, rent, utilities)
- **Equity** - Opening balances, adjustments

### 2. Recording Transfers

```bash
# Receive salary
pecunio transfer 5000 --from Salary --to Checking --category income

# Pay rent
pecunio transfer 1200 --from Checking --to Rent --category housing

# Buy groceries
pecunio transfer 150 --from Checking --to Groceries --category groceries --description "Weekly shopping"

# Record a past transaction
pecunio transfer 50 --from Checking --to Groceries --date 2024-01-15
```

### 3. Checking Balances

```bash
# Check all balances
pecunio balance

# Check specific wallet
pecunio balance Checking

# List recent transfers
pecunio transfers --limit 10

# Filter by category
pecunio transfers --category groceries

# Filter by date range
pecunio transfers --from-date 2024-01-01 --to-date 2024-01-31
```

### 4. Budgeting

```bash
# Create a monthly grocery budget
pecunio budget create GroceryBudget --category groceries --amount 600 --period monthly

# Check budget status
pecunio budget status

# List all budgets
pecunio budget list
```

### 5. Scheduled Transfers (Recurring)

```bash
# Create monthly salary (auto-executes on the 1st)
pecunio scheduled create Salary \
  --from Salary --to Checking \
  --amount 5000 --pattern monthly \
  --start-date 2024-01-01 \
  --category income

# Create monthly rent payment
pecunio scheduled create Rent \
  --from Checking --to Rent \
  --amount 1200 --pattern monthly \
  --start-date 2024-01-05 \
  --category housing

# List scheduled transfers
pecunio scheduled list

# Scheduled transfers execute automatically on every CLI command!
# Use -v to see what was auto-executed
pecunio -v balance
```

### 6. Forecasting

```bash
# Forecast 3 months ahead (based on scheduled transfers)
pecunio forecast

# Forecast 6 months
pecunio forecast --months 6
```

### 7. Reporting

```bash
# Category spending report
pecunio report spending

# With custom date range
pecunio report spending --from 2024-01-01 --to 2024-01-31

# Income vs expense analysis
pecunio report income-expense

# Cash flow by month
pecunio report cashflow --period monthly

# Net worth summary
pecunio report net-worth

# Period comparison (this month vs last month)
pecunio report compare --period monthly

# Export as JSON
pecunio report spending --format json

# Export as CSV
pecunio report spending --format csv
```

### 8. Import/Export

```bash
# Export all transfers to CSV
pecunio export transfers -o transfers.csv

# Export balances
pecunio export balances -o balances.csv

# Full database backup
pecunio export full -o backup.json

# Import transfers from CSV
pecunio import transfers -i bank_export.csv --create-wallets

# Validate import without executing
pecunio import transfers -i data.csv --validate
```

## Example Use Cases

### Use Case 1: Monthly Budget Tracking

```bash
# Setup
pecunio wallet create Checking --type asset
pecunio wallet create Salary --type income
pecunio wallet create Groceries --type expense
pecunio wallet create Dining --type expense
pecunio wallet create Entertainment --type expense

# Create budgets
pecunio budget create Food --category groceries --amount 600 --period monthly
pecunio budget create DiningOut --category dining --amount 200 --period monthly
pecunio budget create Fun --category entertainment --amount 150 --period monthly

# Record spending throughout the month
pecunio transfer 150 --from Checking --to Groceries --category groceries
pecunio transfer 45 --from Checking --to Dining --category dining
pecunio transfer 30 --from Checking --to Entertainment --category entertainment

# Check budget status anytime
pecunio budget status
```

### Use Case 2: Salary & Bills Automation

```bash
# Setup recurring transfers (execute automatically!)
pecunio scheduled create MonthlySalary \
  --from Salary --to Checking \
  --amount 5000 --pattern monthly \
  --start-date 2024-01-01

pecunio scheduled create Rent \
  --from Checking --to Rent \
  --amount 1200 --pattern monthly \
  --start-date 2024-01-05

pecunio scheduled create Internet \
  --from Checking --to Utilities \
  --amount 60 --pattern monthly \
  --start-date 2024-01-10

# They'll execute automatically when due
# Check what's scheduled
pecunio scheduled list

# Forecast your balance
pecunio forecast --months 6
```

### Use Case 3: Credit Card Tracking

```bash
# Setup
pecunio wallet create Checking --type asset
pecunio wallet create CreditCard --type liability

# Borrow from credit card (increases debt)
pecunio transfer 500 --from CreditCard --to Checking

# Pay credit card bill (reduces debt)
pecunio transfer 500 --from Checking --to CreditCard

# Check net worth (assets - liabilities)
pecunio report net-worth
```

### Use Case 4: Expense Analysis

```bash
# After recording transactions for a month, analyze spending
pecunio report spending --from 2024-01-01 --to 2024-01-31

# Compare this month vs last month
pecunio report compare --period monthly

# See cash flow over time
pecunio report cashflow --period monthly

# Export for spreadsheet analysis
pecunio export transfers -o january.csv
```

### Use Case 5: Bank Import & Reconciliation

```bash
# Export your bank transactions as CSV
# Import into Pecunio
pecunio import transfers -i bank_export.csv \
  --create-wallets \
  --skip-duplicates

# Verify imported data
pecunio transfers --limit 50

# Check integrity
pecunio check
```

## Command Reference

### Global Flags
- `-d, --database <PATH>` - Database file path (default: pecunio.db)
- `-v, --verbose` - Enable verbose output (shows auto-executed transfers)

### Commands

**Initialization:**
- `pecunio init` - Initialize a new database

**Wallet Management:**
- `pecunio wallet create <NAME> --type <TYPE>` - Create wallet
- `pecunio wallet list` - List all wallets
- `pecunio wallet show <NAME>` - Show wallet details
- `pecunio wallet archive <NAME>` - Archive wallet

**Transfers:**
- `pecunio transfer <AMOUNT> --from <WALLET> --to <WALLET>` - Record transfer
- `pecunio transfers` - List transfers
- `pecunio show <ID>` - Show transfer details
- `pecunio reverse <ID>` - Reverse a transfer
- `pecunio balance [WALLET]` - Show balance(s)

**Budgets:**
- `pecunio budget create <NAME> --category <CAT> --amount <AMT> --period <PERIOD>`
- `pecunio budget list` - List budgets
- `pecunio budget status` - Show budget status
- `pecunio budget delete <NAME>` - Delete budget

**Scheduled Transfers:**
- `pecunio scheduled create <NAME> --from <WALLET> --to <WALLET> --amount <AMT> --pattern <PATTERN> --start-date <DATE>`
- `pecunio scheduled list` - List scheduled transfers
- `pecunio scheduled show <NAME>` - Show details
- `pecunio scheduled pause/resume <NAME>` - Pause/resume
- `pecunio scheduled delete <NAME>` - Delete
- `pecunio scheduled execute` - Manually execute due transfers

**Forecasting:**
- `pecunio forecast [--months N]` - Project future balances

**Reporting:**
- `pecunio report spending` - Category spending breakdown
- `pecunio report income-expense` - Income vs expense analysis
- `pecunio report cashflow` - Cash flow by period
- `pecunio report net-worth` - Net worth summary
- `pecunio report compare` - Period comparison

**Import/Export:**
- `pecunio export <TYPE> -o <FILE>` - Export data (types: transfers, balances, budgets, scheduled, full)
- `pecunio import <TYPE> -i <FILE>` - Import data (types: transfers, full)

**Utility:**
- `pecunio check` - Verify ledger integrity

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test phase5_reporting_test

# Run with output
cargo test -- --nocapture
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run without installing
cargo run -- balance
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - See LICENSE file for details

## Roadmap

**Possible future enhancements:**
- Multi-currency support with exchange rates
- Advanced forecasting with trend analysis
- Bank API integrations (Plaid, Teller)
- TUI (Terminal UI) interface
- Web UI with local-first sync
- Mobile apps

## Support

- **Bug Reports:** Open an issue on GitHub
- **Feature Requests:** Open an issue with the "enhancement" label
- **Documentation:** Check the help text: `pecunio --help`

---

**built with ❤️** by Andrea _pavonz_ Pavoni.
