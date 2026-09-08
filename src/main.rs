extern crate core;

mod branch_rules;
mod commands;
mod config;
mod errors;
mod git_service;
mod http_service;
mod models;
mod pipelines;
mod versioning_service;

use errors::{messages, DefaultError, Result, ResultExt};
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
        Err(error) => return Err(DefaultError::new(messages::LOAD_ENV).with_source(error)),
    }
    env_logger::try_init().context(messages::INIT_LOGGER)?;
    commands::run()
}
