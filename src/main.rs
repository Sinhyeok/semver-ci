extern crate core;

mod commands;
mod config;
mod git_service;
mod http_service;
mod pipelines;
mod release;
mod semantic_version;

use dotenv::dotenv;

fn main() {
    dotenv().ok();
    commands::run();
}
