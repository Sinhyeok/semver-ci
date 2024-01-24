use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn branch_name(&self) -> String {
        env::var("GITHUB_REF_NAME").unwrap_or_else(|e| panic!("{}: \"GITHUB_REF_NAME\"", e))
    }

    fn short_commit_sha(&self) -> String {
        let commit_sha = env::var("GITHUB_SHA").unwrap_or_else(|e| panic!("{}: \"GITHUB_SHA\"", e));
        commit_sha[0..8].to_owned()
    }

    fn git_username(&self) -> String {
        env::var("GITHUB_ACTOR").unwrap_or_else(|e| panic!("{}: \"GITHUB_ACTOR\"", e))
    }

    fn git_token(&self) -> String {
        env::var("GITHUB_TOKEN").unwrap_or_else(|e| panic!("{}: \"GITHUB_TOKEN\"", e))
    }
}
