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
