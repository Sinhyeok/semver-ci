use crate::pipeline::Pipeline;
use std::env;

pub(crate) struct GithubAction;

impl Pipeline for GithubAction {
    fn branch_name(&self) -> String {
        env::var("GITHUB_REF_NAME").unwrap_or_else(|e| panic!("{}", e))
    }

    fn short_commit_sha(&self) -> String {
        let commit_sha = env::var("GITHUB_SHA").unwrap_or_else(|e| panic!("{}", e));
        commit_sha[0..8].to_owned()
    }
}
