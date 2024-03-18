mod git_repo;
mod github_actions;
mod gitlab_ci;

use crate::pipelines::git_repo::GitRepo;
use crate::pipelines::github_actions::{GithubActions, GITHUB_ACTIONS};
use crate::pipelines::gitlab_ci::{GitlabCI, GITLAB_CI};
use crate::release::Release;
use serde_json::Value;
use std::collections::HashMap;
use std::env;

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
    fn create_release(&self, _release: &Release) -> HashMap<String, Value> {
        panic!("Not supported pipeline: {}", self.name())
    }
    fn env_var(&self, name: &str) -> String {
        env::var(name).unwrap_or_else(|e| panic!("{}: \"{}\"", e, name))
    }
    fn env_var_or(&self, name: &str, default: &str) -> String {
        env::var(name).unwrap_or(default.to_string())
    }
    fn target_path(&self) -> String {
        self.env_var_or("CLONE_TARGET_PATH", ".")
    }
}

enum Pipelines {
    GithubActions(GithubActions),
    GitlabCI(GitlabCI),
    GitRepo(GitRepo),
}

fn pipelines() -> Pipelines {
    if env::var(GITHUB_ACTIONS).map_or(false, |v| v == "true") {
        eprintln!("on GITHUB_ACTIONS");
        Pipelines::GithubActions(GithubActions)
    } else if env::var(GITLAB_CI).map_or(false, |v| v == "true") {
        eprintln!("on GITLAB_CI");
        Pipelines::GitlabCI(GitlabCI)
    } else {
        eprintln!("on GIT Repo");
        Pipelines::GitRepo(GitRepo)
    }
}

pub(crate) struct PipelineInfo {
    pub branch_name: String,
    pub short_commit_sha: String,
    pub git_username: String,
    pub git_email: String,
    pub git_token: String,
    pub force_fetch_tags: bool,
    pub target_path: String,
}

impl PipelineInfo {
    fn new(pipeline: &dyn Pipeline, init: bool) -> PipelineInfo {
        if init {
            pipeline.init();
        }

        PipelineInfo {
            branch_name: pipeline.branch_name(),
            short_commit_sha: pipeline.short_commit_sha(),
            git_username: pipeline.git_username(),
            git_email: pipeline.git_email(),
            git_token: pipeline.git_token(),
            force_fetch_tags: pipeline.force_fetch_tags(),
            target_path: pipeline.target_path(),
        }
    }
}

pub(crate) fn pipeline_info(init: bool) -> PipelineInfo {
    match pipelines() {
        Pipelines::GithubActions(p) => PipelineInfo::new(&p, init),
        Pipelines::GitlabCI(p) => PipelineInfo::new(&p, init),
        Pipelines::GitRepo(p) => PipelineInfo::new(&p, init),
    }
}

pub(crate) fn create_release(release: &Release) -> HashMap<String, Value> {
    match pipelines() {
        Pipelines::GithubActions(p) => p.create_release(release),
        Pipelines::GitlabCI(p) => p.create_release(release),
        Pipelines::GitRepo(p) => p.create_release(release),
    }
}
