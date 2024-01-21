use crate::git_service;
use crate::pipelines::Pipeline;

pub(crate) struct GitRepo;

impl Pipeline for GitRepo {
    fn branch_name(&self) -> String {
        git_service::branch_name().unwrap_or_else(|e| panic!("{}", e))
    }

    fn short_commit_sha(&self) -> String {
        "git_repo_short_commit_sha".to_string()
    }
}
