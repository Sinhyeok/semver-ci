mod git_repo;
mod github_actions;
mod gitlab_ci;

use crate::config;
use crate::default_error::{DefaultError, Result};
use crate::error_messages as messages;
use crate::pipelines::git_repo::GitRepo;
use crate::pipelines::github_actions::{GithubActions, GITHUB_ACTIONS};
use crate::pipelines::gitlab_ci::{GitlabCI, GITLAB_CI};
use crate::release::Release;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) trait Pipeline {
    fn init(&self) -> Result<()> {
        Ok(())
    }
    fn name(&self) -> String;
    fn branch_name(&self) -> Result<String>;
    fn short_commit_sha(&self) -> Result<String>;
    fn target_commit(&self) -> Result<String> {
        Ok("HEAD".to_string())
    }
    fn git_username(&self) -> Result<String>;
    fn git_email(&self) -> Result<String>;
    fn git_token(&self) -> Result<String>;
    fn force_fetch_tags(&self) -> Result<bool> {
        Ok(true)
    }
    fn create_release(&self, _release: &Release) -> Result<HashMap<String, Value>> {
        Err(DefaultError::new(messages::unsupported_pipeline(
            &self.name(),
        )))
    }

    fn info(&self) -> Result<PipelineInfo> {
        Ok(PipelineInfo {
            branch_name: self.branch_name()?,
            short_commit_sha: self.short_commit_sha()?,
            git_username: self.git_username()?,
            git_email: self.git_email()?,
            git_token: self.git_token()?,
            force_fetch_tags: self.force_fetch_tags()?,
        })
    }
}

pub(crate) fn current_pipeline() -> Result<&'static dyn Pipeline> {
    let pipeline = if config::optional_env_var(GITHUB_ACTIONS)?.is_some_and(|v| v == "true") {
        &GithubActions as &dyn Pipeline
    } else if config::optional_env_var(GITLAB_CI)?.is_some_and(|v| v == "true") {
        &GitlabCI as &dyn Pipeline
    } else {
        &GitRepo as &dyn Pipeline
    };

    eprintln!("on {}", pipeline.name());

    Ok(pipeline)
}

pub(crate) struct PipelineInfo {
    pub branch_name: String,
    pub short_commit_sha: String,
    pub git_username: String,
    pub git_email: String,
    pub git_token: String,
    pub force_fetch_tags: bool,
}
