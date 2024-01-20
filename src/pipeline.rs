use crate::git_repo::GitRepo;
use crate::github_action::GithubActions;
use crate::gitlab_ci::GitlabCI;
use std::env;

pub(crate) trait Pipeline {
    fn branch_name(&self) -> String;
    fn short_commit_sha(&self) -> String;
}

pub(crate) enum PipelineType {
    GithubActions(GithubActions),
    GitlabCI(GitlabCI),
    GitRepo(GitRepo),
}

const GITLAB_CI: &str = "GITLAB_CI";
const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

pub(crate) fn pipeline_type() -> PipelineType {
    if env::var(GITLAB_CI).map_or(false, |v| v == "true") {
        PipelineType::GitlabCI(GitlabCI)
    } else if env::var(GITHUB_ACTIONS).map_or(false, |v| v == "true") {
        PipelineType::GithubActions(GithubActions)
    } else {
        PipelineType::GitRepo(GitRepo)
    }
}
