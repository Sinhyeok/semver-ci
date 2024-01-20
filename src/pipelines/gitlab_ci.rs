use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GitlabCI;

pub const GITLAB_CI: &str = "GITLAB_CI";

impl Pipeline for GitlabCI {
    fn branch_name(&self) -> String {
        env::var("CI_COMMIT_BRANCH").unwrap_or_else(|e| panic!("{}", e))
    }

    fn short_commit_sha(&self) -> String {
        env::var("CI_COMMIT_SHORT_SHA").unwrap_or_else(|e| panic!("{}", e))
    }
}
