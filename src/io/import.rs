use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use std::io::Read;

use crate::application::LedgerService;
use crate::domain::{parse_cents, WalletType};
use crate::io::export::DatabaseSnapshot;

/// Result of an import operation
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<ImportError>,
}

/// Error that occurred during import
#[derive(Debug, Clone)]
pub struct ImportError {
    pub line: usize,
    pub field: Option<String>,
    pub error: String,
}

/// Options for import operations
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub dry_run: bool,
    pub skip_duplicates: bool,
    pub create_missing_wallets: bool,
    pub validate_only: bool,
}

/// Importer for loading data into the ledger
pub struct Importer<'a> {
    service: &'a LedgerService,
}

impl<'a> Importer<'a> {
    pub fn new(service: &'a LedgerService) -> Self {
        Self { service }
    }

    /// Import transfers from CSV
    pub async fn import_transfers_csv<R: Read>(
        &self,
        reader: R,
        options: ImportOptions,
    ) -> Result<ImportResult> {
        let mut csv_reader = csv::Reader::from_reader(reader);
        let mut imported = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        for (line_num, result) in csv_reader.records().enumerate() {
            let line = line_num + 2; // +2 for header and 0-indexing

            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    errors.push(ImportError {
                        line,
                        field: None,
                        error: format!("CSV parse error: {}", e),
                    });
                    continue;
                }
            };

            // Parse CSV record
            let from_wallet = record.get(3).unwrap_or("");
            let to_wallet = record.get(4).unwrap_or("");
            let amount_str = record.get(5).unwrap_or("");
            let timestamp_str = record.get(2).unwrap_or("");
            let description = record.get(6).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            let category = record.get(7).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });

            // Validate and parse
            let amount_cents = match parse_cents(amount_str) {
                Ok(a) => a,
                Err(e) => {
                    errors.push(ImportError {
                        line,
                        field: Some("amount_cents".to_string()),
                        error: format!("Invalid amount: {}", e),
                    });
                    continue;
                }
            };

            let timestamp = match parse_timestamp(timestamp_str) {
                Ok(ts) => ts,
                Err(e) => {
                    errors.push(ImportError {
                        line,
                        field: Some("timestamp".to_string()),
                        error: format!("Invalid timestamp: {}", e),
                    });
                    continue;
                }
            };

            // Validate wallets exist (or create them)
            if options.create_missing_wallets {
                // Try to create wallets if they don't exist
                if let Err(e) = ensure_wallet_exists(self.service, from_wallet).await {
                    errors.push(ImportError {
                        line,
                        field: Some("from_wallet".to_string()),
                        error: format!("Wallet error: {}", e),
                    });
                    continue;
                }
                if let Err(e) = ensure_wallet_exists(self.service, to_wallet).await {
                    errors.push(ImportError {
                        line,
                        field: Some("to_wallet".to_string()),
                        error: format!("Wallet error: {}", e),
                    });
                    continue;
                }
            }

            // Skip actual import if dry run or validate only
            if options.dry_run || options.validate_only {
                imported += 1;
                continue;
            }

            // Import the transfer
            match self
                .service
                .record_transfer(
                    from_wallet,
                    to_wallet,
                    amount_cents,
                    timestamp,
                    description.clone(),
                    category.clone(),
                    true, // force (allow negative balances during import)
                )
                .await
            {
                Ok(_) => {
                    imported += 1;
                }
                Err(e) => {
                    if options.skip_duplicates {
                        skipped += 1;
                    } else {
                        errors.push(ImportError {
                            line,
                            field: None,
                            error: format!("Transfer creation failed: {}", e),
                        });
                    }
                }
            }
        }

        Ok(ImportResult {
            imported,
            skipped,
            errors,
        })
    }

    /// Import full database from JSON snapshot
    pub async fn import_full_json<R: Read>(
        &self,
        reader: R,
        options: ImportOptions,
    ) -> Result<ImportResult> {
        let snapshot: DatabaseSnapshot = serde_json::from_reader(reader)?;

        let imported = 0;
        let skipped = 0;
        let mut errors = Vec::new();

        if options.validate_only {
            // Just validate the JSON structure
            return Ok(ImportResult {
                imported: snapshot.wallets.len()
                    + snapshot.transfers.len()
                    + snapshot.budgets.len()
                    + snapshot.scheduled_transfers.len(),
                skipped: 0,
                errors,
            });
        }

        // Note: Full import would require more complex logic to handle:
        // - Creating wallets first
        // - Then transfers (respecting sequence)
        // - Then budgets
        // - Then scheduled transfers
        // For now, we'll return an error indicating this needs manual handling

        errors.push(ImportError {
            line: 0,
            field: None,
            error: "Full JSON import not yet implemented. Use CSV import for transfers."
                .to_string(),
        });

        Ok(ImportResult {
            imported,
            skipped,
            errors,
        })
    }
}

// Helper function to parse timestamp
fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    // Try RFC3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try YYYY-MM-DD format
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    anyhow::bail!("Invalid timestamp format: {}", s)
}

// Helper to ensure wallet exists
async fn ensure_wallet_exists(service: &LedgerService, name: &str) -> Result<()> {
    // Check if wallet already exists
    if service.get_wallet_info(name).await.is_ok() {
        return Ok(());
    }

    // Create a default expense wallet (user can change type later)
    service
        .create_wallet(
            name.to_string(),
            WalletType::Expense,
            "USD".to_string(),
            Some("Auto-created during import".to_string()),
        )
        .await?;

    Ok(())
}
