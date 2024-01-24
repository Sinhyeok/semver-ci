use crate::git_service;
use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GitRepo;

impl Pipeline for GitRepo {
    fn branch_name(&self) -> String {
        git_service::branch_name()
            .unwrap_or_else(|e| panic!("Failed to retrieve branch_name: {}", e))
    }

    fn short_commit_sha(&self) -> String {
        git_service::short_commit_sha()
            .unwrap_or_else(|e| panic!("Failed to retrieve short_commit_sha: {}", e))
    }

    fn git_username(&self) -> String {
        env::var("GIT_USERNAME").unwrap_or_else(|e| panic!("{}: \"GIT_USERNAME\"", e))
    }

    fn git_token(&self) -> String {
        env::var("GIT_TOKEN").unwrap_or_else(|e| panic!("{}: \"GIT_TOKEN\"", e))
    }

    fn force_fetch_tags(&self) -> bool {
        env::var("FORCE_FETCH_TAGS")
            .unwrap_or("false".to_string())
            .parse()
            .unwrap_or_else(|e| panic!("{}: \"FORCE_FETCH_TAGS\"", e))
    }
}
