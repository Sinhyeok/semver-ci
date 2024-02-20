mod git_repo;
mod github_actions;
mod gitlab_ci;

use crate::pipelines::git_repo::GitRepo;
use crate::pipelines::github_actions::{GithubActions, GITHUB_ACTIONS};
use crate::pipelines::gitlab_ci::{GitlabCI, GITLAB_CI};
use std::env;

pub(crate) trait Pipeline {
    fn init(&self) {}
    fn branch_name(&self) -> String;
    fn short_commit_sha(&self) -> String;
    fn git_username(&self) -> String;
    fn git_email(&self) -> String;
    fn git_token(&self) -> String;
    fn force_fetch_tags(&self) -> bool {
        true
    }
}

enum Pipelines {
    GithubActions(GithubActions),
    GitlabCI(GitlabCI),
    GitRepo(GitRepo),
}

fn pipeline() -> Pipelines {
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
        }
    }
}

pub(crate) fn pipeline_info(init: bool) -> PipelineInfo {
    match pipeline() {
        Pipelines::GithubActions(p) => PipelineInfo::new(&p, init),
        Pipelines::GitlabCI(p) => PipelineInfo::new(&p, init),
        Pipelines::GitRepo(p) => PipelineInfo::new(&p, init),
    }
}
