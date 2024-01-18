use crate::pipeline::Pipeline;

pub(crate) struct GitRepo;

impl Pipeline for GitRepo {
    fn branch_name(&self) -> String {
        "git_repo_branch_name".to_string()
    }

    fn short_commit_sha(&self) -> String {
        "git_repo_short_commit_sha".to_string()
    }
}
