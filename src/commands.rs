use clap::{Args, Parser, Subcommand};

mod version_command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    Version(VersionArgs),
}

#[derive(Args)]
pub(crate) struct VersionArgs {
    #[arg(short, long, default_value = "minor")]
    pub(crate) scope: String,
}

pub(crate) fn run() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Version(args) => version_command::run(args),
    }
}
