use std::env;

pub(crate) fn env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|e| panic!("{}: \"{}\"", e, name))
}

pub(crate) fn env_var_or(name: &str, default: &str) -> String {
    env::var(name).unwrap_or(default.to_string())
}

pub(crate) fn clone_target_path() -> String {
    env::var("CLONE_TARGET_PATH").unwrap_or(".".to_string())
}

fn environment() -> String {
    env::var("ENVIRONMENT").unwrap_or("production".to_string())
}

pub(crate) fn is_production() -> bool {
    environment() == "production"
}

pub(crate) fn is_test() -> bool {
    environment() == "test"
}
