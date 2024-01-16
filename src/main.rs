mod git_service;
mod semantic_version;
mod version_command;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Version(VersionArgs),
}

#[derive(Args)]
struct VersionArgs {
    #[arg(short, long, default_value = "minor")]
    scope: String,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Version(args) => {
            version_command::run(args);
        }
    }
}
