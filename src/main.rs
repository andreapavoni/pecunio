use anyhow::Result;
use clap::Parser;
use pecunio::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().await
}
