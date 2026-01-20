use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::application::LedgerService;
use crate::domain::{Budget, ScheduledTransfer, Transfer, Wallet};

/// Database snapshot for full export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSnapshot {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub wallets: Vec<Wallet>,
    pub transfers: Vec<Transfer>,
    pub budgets: Vec<Budget>,
    pub scheduled_transfers: Vec<ScheduledTransfer>,
}

/// Exporter for converting ledger data to various formats
pub struct Exporter<'a> {
    service: &'a LedgerService,
}

impl<'a> Exporter<'a> {
    pub fn new(service: &'a LedgerService) -> Self {
        Self { service }
    }

    /// Export transfers to CSV format
    pub async fn export_transfers_csv<W: Write>(&self, writer: W) -> Result<usize> {
        let transfers = self.service.list_all_transfers().await?;
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        csv_writer.write_record(&[
            "id",
            "sequence",
            "timestamp",
            "from_wallet",
            "to_wallet",
            "amount_cents",
            "description",
            "category",
            "tags",
            "reverses",
            "external_ref",
        ])?;

        let mut count = 0;
        for transfer in &transfers {
            // Get wallet names
            let from_wallet = self.service.get_wallet_by_id(transfer.from_wallet).await?;
            let to_wallet = self.service.get_wallet_by_id(transfer.to_wallet).await?;

            csv_writer.write_record(&[
                transfer.id.to_string(),
                transfer.sequence.to_string(),
                transfer.timestamp.to_rfc3339(),
                from_wallet.name,
                to_wallet.name,
                transfer.amount_cents.to_string(),
                transfer.description.clone().unwrap_or_default(),
                transfer.category.clone().unwrap_or_default(),
                transfer.tags.join(";"),
                transfer
                    .reverses
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                transfer.external_ref.clone().unwrap_or_default(),
            ])?;
            count += 1;
        }

        csv_writer.flush()?;
        Ok(count)
    }

    /// Export balances to CSV format
    pub async fn export_balances_csv<W: Write>(&self, writer: W) -> Result<usize> {
        let wallets = self.service.list_wallets(false).await?;
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        csv_writer.write_record(&["wallet", "type", "currency", "balance"])?;

        let mut count = 0;
        for wallet in &wallets {
            let balance = self.service.get_balance(&wallet.name).await?;
            csv_writer.write_record(&[
                &wallet.name,
                wallet.wallet_type.as_str(),
                &wallet.currency,
                &balance.balance.to_string(),
            ])?;
            count += 1;
        }

        csv_writer.flush()?;
        Ok(count)
    }

    /// Export budgets to CSV format
    pub async fn export_budgets_csv<W: Write>(&self, writer: W) -> Result<usize> {
        let budgets = self.service.list_budgets().await?;
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        csv_writer.write_record(&["name", "category", "amount_cents", "period"])?;

        let mut count = 0;
        for budget in &budgets {
            csv_writer.write_record(&[
                &budget.name,
                &budget.category,
                &budget.amount_cents.to_string(),
                budget.period_type.as_str(),
            ])?;
            count += 1;
        }

        csv_writer.flush()?;
        Ok(count)
    }

    /// Export scheduled transfers to CSV format
    pub async fn export_scheduled_csv<W: Write>(&self, writer: W) -> Result<usize> {
        let scheduled = self.service.list_scheduled_transfers(true).await?;
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        csv_writer.write_record(&[
            "name",
            "from_wallet",
            "to_wallet",
            "amount_cents",
            "pattern",
            "start_date",
            "end_date",
            "description",
            "category",
            "status",
        ])?;

        let mut count = 0;
        for st in &scheduled {
            let from_wallet = self.service.get_wallet_by_id(st.from_wallet).await?;
            let to_wallet = self.service.get_wallet_by_id(st.to_wallet).await?;

            csv_writer.write_record(&[
                &st.name,
                &from_wallet.name,
                &to_wallet.name,
                &st.amount_cents.to_string(),
                st.pattern.as_str(),
                &st.start_date.to_rfc3339(),
                &st.end_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
                &st.description.clone().unwrap_or_default(),
                &st.category.clone().unwrap_or_default(),
                st.status.as_str(),
            ])?;
            count += 1;
        }

        csv_writer.flush()?;
        Ok(count)
    }

    /// Export full database as JSON snapshot
    pub async fn export_full_json<W: Write>(&self, mut writer: W) -> Result<DatabaseSnapshot> {
        let wallets = self.service.list_wallets(true).await?;
        let transfers = self.service.list_all_transfers().await?;
        let budgets = self.service.list_budgets().await?;
        let scheduled_transfers = self.service.list_scheduled_transfers(true).await?;

        let snapshot = DatabaseSnapshot {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now(),
            wallets,
            transfers,
            budgets,
            scheduled_transfers,
        };

        let json = serde_json::to_string_pretty(&snapshot)?;
        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(snapshot)
    }
}
