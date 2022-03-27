use clap::Parser;

use transaction_engine::TransactionEngine;

/// A cli interface to the transaction engine
#[derive(Debug, Parser)]
#[clap(version)]
struct Args {
    /// The path to the transaction CSV file
    filename: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(args.filename)?;
    let mut engine = TransactionEngine::new();

    for transaction in reader.deserialize() {
        // failed transactions are just ignored
        let _ = engine.handle_transaction(transaction?);
    }

    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(std::io::stdout());

    for account in engine.accounts().values() {
        writer.serialize(account)?;
    }

    Ok(())
}
