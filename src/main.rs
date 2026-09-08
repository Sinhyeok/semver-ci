extern crate core;

mod branch_rules;
mod commands;
mod config;
mod default_error;
mod git_service;
mod http_service;
mod pipelines;
mod release;
mod semantic_version;
mod versioning_service;

use dotenv::dotenv;

fn main() {
    dotenv().ok();
    env_logger::init();

    commands::run().unwrap_or_else(|e| match e.source() {
        Some(source) => panic!("{}\n    Caused by: {}", e, source),
        None => panic!("{}", e),
    });
}
