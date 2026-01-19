# Pecunio

A local-first personal finance tool based on a transfer-based ledger with event sourcing.

## Philosophy

- **Transfer-based ledger**: Every financial event is an atomic transfer of money from one wallet to another. No classical double-entry accounting (debit/credit), just simple transfers.
- **Event sourcing**: The ledger is append-only and serves as the single source of truth. Wallet balances are always derived, never stored.
- **Local-first**: Your data stays on your machine in a SQLite database.
- **CLI-first**: Designed for terminal users who prefer keyboard over mouse.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/pecunio`.

## Quick Start

```bash
# Initialize the database
pecunio init

# Create some wallets
pecunio wallet create "Checking" --type asset
pecunio wallet create "Savings" --type asset
pecunio wallet create "Credit Card" --type liability
pecunio wallet create "Salary" --type income
pecunio wallet create "Groceries" --type expense
pecunio wallet create "Utilities" --type expense

# Record your salary
pecunio transfer 5000 --from Salary --to Checking -d "January salary"

# Pay some bills
pecunio transfer 150 --from Checking --to Utilities -d "Electric bill" -c utilities
pecunio transfer 85.50 --from Checking --to Groceries -d "Weekly groceries" -c groceries

# Use credit card
pecunio transfer 45 --from "Credit Card" --to Groceries -d "Coffee and snacks" -c groceries

# Pay off credit card
pecunio transfer 45 --from Checking --to "Credit Card" -d "CC payment"

# Move money to savings
pecunio transfer 500 --from Checking --to Savings -d "Monthly savings"

# Check balances
pecunio balance
```

## Concepts

### Wallet Types

| Type | Description | Examples |
|------|-------------|----------|
| `asset` | Money you own | Bank accounts, cash, investments |
| `liability` | Money you owe | Credit cards, loans |
| `income` | External money sources | Employers, interest, gifts |
| `expense` | External money destinations | Merchants, bills, subscriptions |
| `equity` | Opening balances, adjustments | Used for migration from other systems |

### How Transfers Work

Every transfer moves money from one wallet to another:

```
Transfer: Salary -> Checking, 5000
  - Salary balance:   -5000 (you "owe" income to the system)
  - Checking balance: +5000 (money in your account)
```

The sum of all wallet balances is always zero (closed system).

### Credit Cards

Credit cards are `liability` wallets. When you make a purchase:

```bash
# Purchase increases your debt (CC balance becomes negative)
pecunio transfer 50 --from "Credit Card" --to "Amazon" -d "Books"

# Payment decreases your debt
pecunio transfer 50 --from Checking --to "Credit Card" -d "CC payment"
```

### Categories

Use categories for budgeting and reporting:

```bash
pecunio transfer 120 --from Checking --to "Whole Foods" -c groceries
pecunio transfer 45 --from Checking --to "Shell" -c transportation
```

## Commands

### Database

```bash
pecunio init                  # Create database (default: pecunio.db)
pecunio -d myfinances.db init # Use custom database file
```

### Wallets

```bash
pecunio wallet create "Name" --type asset     # Create wallet
pecunio wallet create "Name" -t liability     # Short form
pecunio wallet create "Name" -t asset -c USD  # Specify currency
pecunio wallet list                           # List all wallets
pecunio wallet list --all                     # Include archived
pecunio wallet archive "Name"                 # Archive a wallet
```

### Transfers

```bash
pecunio transfer 100 --from Source --to Dest              # Basic transfer
pecunio transfer 99.99 --from A --to B -d "Description"   # With description
pecunio transfer 50 --from A --to B -c groceries          # With category
```

### Balances

```bash
pecunio balance              # Show all wallet balances
pecunio balance "Checking"   # Show specific wallet balance
```

### Transaction History

```bash
pecunio transfers                    # List recent transfers (default: 20)
pecunio transfers --limit 50         # Show more
pecunio transfers --wallet Checking  # Filter by wallet
```

## Data Model

### Wallets Table

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| name | TEXT | Unique wallet name |
| wallet_type | TEXT | asset, liability, income, expense, equity |
| currency | TEXT | ISO 4217 code (default: EUR) |
| allow_negative | BOOL | Whether balance can go negative |
| description | TEXT | Optional description |
| created_at | TEXT | ISO 8601 timestamp |
| archived_at | TEXT | Soft delete timestamp |

### Transfers Table

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| sequence | INT | Monotonic ordering |
| from_wallet_id | UUID | Source wallet |
| to_wallet_id | UUID | Destination wallet |
| amount_cents | INT | Amount in cents (always positive) |
| timestamp | TEXT | When transaction occurred |
| recorded_at | TEXT | When we recorded it |
| description | TEXT | Human-readable description |
| category | TEXT | For budgeting/reporting |
| tags | JSON | Additional metadata |
| reverses | UUID | Link to reversed transfer |
| external_ref | TEXT | Bank transaction ID, etc. |

## Design Principles

1. **Money is stored as integer cents** - No floating point precision issues
2. **Balances are always computed** - Never stored, always derived from transfers
3. **Transfers are immutable** - Corrections via reversals, never edits
4. **Append-only ledger** - Full audit trail, supports replay from genesis

## Roadmap

- [x] Phase 1: Minimal viable ledger (transfers, balances, persistence)
- [ ] Phase 2: Wallet management, efficient SQL-based balance queries
- [ ] Phase 3: CLI ergonomics, budget tracking via categories
- [ ] Phase 4: Scheduled/recurring transfers, forecasting
- [ ] Phase 5: Reporting, integrity checks, import/export

## License

MIT
