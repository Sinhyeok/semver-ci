extern crate core;

mod commands;
mod config;
mod default_error;
mod git_service;
mod http_service;
mod pipelines;
mod release;
mod semantic_version;

use dotenv::dotenv;

fn main() {
    dotenv().ok();

    commands::run().unwrap_or_else(|e| match e.source() {
        Some(source) => panic!("{}\nCaused by: {}", e, source),
        None => panic!("{}", e),
    });
}
