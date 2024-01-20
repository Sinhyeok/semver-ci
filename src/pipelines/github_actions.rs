use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn branch_name(&self) -> String {
        env::var("GITHUB_REF_NAME").unwrap_or_else(|e| panic!("{}", e))
    }

    fn short_commit_sha(&self) -> String {
        let commit_sha = env::var("GITHUB_SHA").unwrap_or_else(|e| panic!("{}", e));
        commit_sha[0..8].to_owned()
    }
}
