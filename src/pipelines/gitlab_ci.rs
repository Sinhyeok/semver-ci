use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GitlabCI;

pub const GITLAB_CI: &str = "GITLAB_CI";

impl Pipeline for GitlabCI {
    fn branch_name(&self) -> String {
        env::var("CI_COMMIT_BRANCH").unwrap_or_else(|e| panic!("{}: \"CI_COMMIT_BRANCH\"", e))
    }

    fn short_commit_sha(&self) -> String {
        env::var("CI_COMMIT_SHORT_SHA").unwrap_or_else(|e| panic!("{}: \"CI_COMMIT_SHORT_SHA\"", e))
    }

    fn git_username(&self) -> String {
        "gitlab-ci-token".to_string()
    }

    fn git_token(&self) -> String {
        env::var("CI_JOB_TOKEN").unwrap_or_else(|e| panic!("{}: \"CI_JOB_TOKEN\"", e))
    }
}
