use crate::commands::scope_command::ScopeCommandArgs;
use crate::commands::tag_command::TagCommandArgs;
use clap::{Parser, Subcommand};
use version_command::VersionCommandArgs;

mod scope_command;
mod tag_command;
mod version_command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Version(VersionCommandArgs),
    Scope(ScopeCommandArgs),
    Tag(TagCommandArgs),
}

pub(crate) fn run() {
    match Cli::parse().command {
        Commands::Version(args) => version_command::run(args),
        Commands::Scope(args) => scope_command::run(args),
        Commands::Tag(args) => tag_command::run(args),
    }
}
