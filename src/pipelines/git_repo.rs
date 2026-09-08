use crate::errors::{messages, Result, ResultExt};
use crate::pipelines::Pipeline;
use crate::{config, git_service};

pub(crate) struct GitRepo;

impl Pipeline for GitRepo {
    fn name(&self) -> String {
        "Git Repo".to_string()
    }

    fn branch_name(&self) -> Result<String> {
        git_service::branch_name(&config::clone_target_path()?)
    }

    fn short_commit_sha(&self) -> Result<String> {
        git_service::short_commit_sha(&config::clone_target_path()?)
    }

    fn git_username(&self) -> Result<String> {
        Ok(
            git_service::get_config_value(&config::clone_target_path()?, "user.name")?
                .unwrap_or_default(),
        )
    }

    fn git_email(&self) -> Result<String> {
        Ok(
            git_service::get_config_value(&config::clone_target_path()?, "user.email")?
                .unwrap_or_default(),
        )
    }

    fn git_token(&self) -> Result<String> {
        config::env_var("GIT_TOKEN")
    }

    fn force_fetch_tags(&self) -> Result<bool> {
        let flag = config::env_var_or("FORCE_FETCH_TAGS", "false")?;
        flag.parse()
            .context(messages::invalid_boolean("FORCE_FETCH_TAGS", &flag))
    }
}
