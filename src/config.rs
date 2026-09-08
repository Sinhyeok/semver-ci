use crate::default_error::{DefaultError, Result, ResultExt};
use crate::error_messages as messages;
use std::env;

pub(crate) fn env_var(name: &str) -> Result<String> {
    env::var(name).context(messages::environment(name))
}

pub(crate) fn optional_env_var(name: &str) -> Result<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(DefaultError::new(messages::environment(name)).with_source(error)),
    }
}

pub(crate) fn env_var_or(name: &str, default: &str) -> Result<String> {
    Ok(optional_env_var(name)?.unwrap_or_else(|| default.to_string()))
}

pub(crate) fn clone_target_path() -> Result<String> {
    env_var_or("CLONE_TARGET_PATH", ".")
}

pub(crate) fn is_test() -> Result<bool> {
    Ok(env_var_or("ENVIRONMENT", "production")? == "test")
}
