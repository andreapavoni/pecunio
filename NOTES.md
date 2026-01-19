# Project Planning — Personal Finance Ledger (Plan Mode)

> **Claude, use Plan Mode.**
> We are *only planning the project*.
> Do **not** generate code.

## 1. Context & Intent

You are acting as a **senior software architect and Rust engineer**.

We want to design a **local-first personal finance tool** based on:

* a **transfer-based ledger**
* **event sourcing**
* a **CLI-first interface**
* **SQLite** as embedded storage

The goal of this phase is to obtain a **clear, opinionated roadmap and architectural plan** before any implementation starts.

---

## 2. Core Philosophy

* The system uses an **append-only ledger** as the single source of truth.
* Each ledger entry is an **atomic transfer of money** from one wallet to another.
* There is **no classical double-entry accounting** (no debit/credit accounts).
* Mathematical consistency must be guaranteed at all times.
* Wallet balances are **derived state**, never primary data.
* The system must be **auditable, explainable, and human-readable**.

If an operation cannot be explained clearly in natural language, it is considered invalid.

---

## 3. Domain Concepts

### Wallet

Represents *where money is stored or allocated*.

Can represent:

* bank accounts
* credit cards
* cash
* savings
* budget envelopes
* external entities (employer, merchants)

Properties:

* unique id
* name
* kind (asset, liability, budget, external)
* currency
* allow_negative (e.g. credit cards)
* metadata (JSON)

Wallet balances are **never stored**, only computed.

---

### LedgerEvent

The core domain event.

* Immutable
* Append-only
* Represents a transfer from one wallet to another

Key attributes:

* event_id
* timestamp
* from_wallet_id
* to_wallet_id
* amount
* currency
* description
* category
* tags
* correlation_id (for reversals, batches, related events)

Errors are handled via **compensating events**, never edits or deletes.

---

### ScheduledOperation

Used for **recurring and future-planned operations**.

* Exists only for forecasting and projections
* Does **not** affect real balances
* Can be materialized into real LedgerEvents when executed

---

### Snapshot (optional)

* Cached derived state
* Used for faster startup and validation
* Always rebuildable from the ledger
* Never authoritative

---

## 4. Functional Requirements

The system must support:

* Regular income (salary)
* Variable income
* Fixed and variable expenses
* Recurring expenses (with optional end date)
* One-off expenses
* Credit card spending with delayed settlement
* Budgeting via dedicated wallets
* Spending analysis by category and time range
* Balance overview per wallet
* Forecasting based on scheduled operations
* Integrity checks (sum of wallet balances must remain constant)

---

## 5. Architectural Constraints

* Programming language: **Rust**
* Interface: **CLI-first**
* Storage: **SQLite**

  * append-only friendly
  * SQL schema with limited JSON usage
* Database access: **sqlx**
* Clear separation between:

  * domain (pure logic)
  * application / use cases
  * storage / persistence
  * CLI interface

---

## 6. Event Sourcing Model

* The ledger is the **only source of truth**
* All state is derived from ledger events
* Reversals and corrections are modeled explicitly
* Snapshots are optional optimizations
* The system must support full replay from genesis

---

## 7. CLI Expectations (Conceptual)

Examples of commands the CLI should eventually support:

* create and list wallets
* add ledger events (transfers)
* reverse ledger events
* show wallet balances
* show global balance
* generate forecasts for N months
* generate reports (by category, by period)
* materialize scheduled operations

This section is conceptual; no command syntax is required yet.

---

## 8. What You Should Produce (Planning Only)

In **Plan Mode**, provide:

1. **Validation of the conceptual model**

   * strengths
   * weaknesses
   * suggested improvements or simplifications

2. **High-level project structure**

   * modules / crates
   * responsibilities and boundaries

3. **High-level database design**

   * tables
   * responsibilities
   * relationships
   * append-only considerations

4. **Step-by-step roadmap**

   * Phase 1: minimal viable ledger
   * Phase 2: wallets and balance computation
   * Phase 3: CLI ergonomics
   * Phase 4: scheduling and forecasting
   * Phase 5: reporting, validation, and integrity checks

5. **Key design decisions to lock early**

   * things that are hard to change later
   * common pitfalls to avoid

Be **pragmatic, opinionated, and concrete**.
Avoid over-engineering and unnecessary abstractions.
