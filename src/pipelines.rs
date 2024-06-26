mod git_repo;
mod github_actions;
mod gitlab_ci;

use crate::default_error::DefaultError;
use crate::pipelines::git_repo::GitRepo;
use crate::pipelines::github_actions::{GithubActions, GITHUB_ACTIONS};
use crate::pipelines::gitlab_ci::{GitlabCI, GITLAB_CI};
use crate::release::Release;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::error::Error;

pub(crate) trait Pipeline {
    fn init(&self) {}
    fn name(&self) -> String;
    fn branch_name(&self) -> String;
    fn short_commit_sha(&self) -> String;
    fn git_username(&self) -> String;
    fn git_email(&self) -> String;
    fn git_token(&self) -> String;
    fn force_fetch_tags(&self) -> bool {
        true
    }
    fn create_release(&self, _release: &Release) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        Err(Box::new(DefaultError {
            message: format!("Not supported pipeline: {}", self.name()),
            source: None,
        }))
    }

    fn info(&self) -> PipelineInfo {
        PipelineInfo {
            branch_name: self.branch_name(),
            short_commit_sha: self.short_commit_sha(),
            git_username: self.git_username(),
            git_email: self.git_email(),
            git_token: self.git_token(),
            force_fetch_tags: self.force_fetch_tags(),
        }
    }
}

pub(crate) fn current_pipeline() -> &'static dyn Pipeline {
    let pipeline = if env::var(GITHUB_ACTIONS).map_or(false, |v| v == "true") {
        &GithubActions as &dyn Pipeline
    } else if env::var(GITLAB_CI).map_or(false, |v| v == "true") {
        &GitlabCI as &dyn Pipeline
    } else {
        &GitRepo as &dyn Pipeline
    };

    eprintln!("on {}", pipeline.name());

    pipeline
}

pub(crate) struct PipelineInfo {
    pub branch_name: String,
    pub short_commit_sha: String,
    pub git_username: String,
    pub git_email: String,
    pub git_token: String,
    pub force_fetch_tags: bool,
}
