use clap::{Parser, Subcommand};
use release_command::ReleaseCommandArgs;
use scope_command::ScopeCommandArgs;
use std::error::Error;
use tag_command::TagCommandArgs;
use version_command::VersionCommandArgs;

mod release_command;
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
    Release(ReleaseCommandArgs),
}

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Commands::Version(args) => version_command::run(args),
        Commands::Scope(args) => scope_command::run(args),
        Commands::Tag(args) => tag_command::run(args),
        Commands::Release(args) => release_command::run(args),
    }
}
