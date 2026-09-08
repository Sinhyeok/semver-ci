extern crate core;

mod branch_rules;
mod commands;
mod config;
mod default_error;
mod error_messages;
mod git_service;
mod http_service;
mod pipelines;
mod release;
mod release_target;
mod semantic_version;
mod versioning_service;

use default_error::{Result, ResultExt};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.report());
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    match dotenv::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(error) => {
            return Err(
                default_error::DefaultError::new(error_messages::LOAD_ENV).with_source(error)
            )
        }
    }
    env_logger::try_init().context(error_messages::INIT_LOGGER)?;
    commands::run()
}
