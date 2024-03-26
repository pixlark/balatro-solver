mod stats;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: CliCommands,
}

#[derive(Debug, Subcommand)]
enum CliCommands {
    /// Generate statistics
    Stats {
        #[command(subcommand)]
        command: stats::CliCommands,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        CliCommands::Stats { command } => stats::run(command),
    }
}
