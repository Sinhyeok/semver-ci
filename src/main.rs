extern crate core;

mod commands;
mod git_service;
mod pipelines;
mod semantic_version;

use dotenv::dotenv;

fn main() {
    dotenv().ok();
    commands::run();
}
