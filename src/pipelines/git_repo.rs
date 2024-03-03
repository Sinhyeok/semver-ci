use crate::git_service;
use crate::pipelines::Pipeline;

pub(crate) struct GitRepo;

impl Pipeline for GitRepo {
    fn name(&self) -> String {
        "Git Repo".to_string()
    }

    fn branch_name(&self) -> String {
        git_service::branch_name()
            .unwrap_or_else(|e| panic!("Failed to retrieve branch_name: {}", e))
    }

    fn short_commit_sha(&self) -> String {
        git_service::short_commit_sha()
            .unwrap_or_else(|e| panic!("Failed to retrieve short_commit_sha: {}", e))
    }

    fn git_username(&self) -> String {
        git_service::get_config_value("user.name").unwrap_or("".to_string())
    }

    fn git_email(&self) -> String {
        git_service::get_config_value("user.email").unwrap_or("".to_string())
    }

    fn git_token(&self) -> String {
        self.env_var("GIT_TOKEN")
    }

    fn force_fetch_tags(&self) -> bool {
        let flag = self.env_var_or("FORCE_FETCH_TAGS", "false");
        flag.parse()
            .unwrap_or_else(|e| panic!("{}\nFORCE_FETCH_TAGS: {}", e, flag))
    }
}
