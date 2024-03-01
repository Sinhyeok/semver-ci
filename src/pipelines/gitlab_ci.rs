use crate::git_service;
use crate::pipelines::Pipeline;
use crate::release::Release;

pub(crate) struct GitlabCI;

pub const GITLAB_CI: &str = "GITLAB_CI";

impl Pipeline for GitlabCI {
    fn init(&self) {
        let project_url = self.env_var("CI_PROJECT_URL");
        git_service::set_config_value("remote.origin.pushurl", &format!("{}.git", project_url))
            .unwrap_or_else(|e| panic!("{}", e));
    }

    fn name(&self) -> String {
        "GitLab CI".to_string()
    }

    fn branch_name(&self) -> String {
        self.env_var("CI_COMMIT_BRANCH")
    }

    fn short_commit_sha(&self) -> String {
        self.env_var("CI_COMMIT_SHORT_SHA")
    }

    fn git_username(&self) -> String {
        "gitlab-ci-token".to_string()
    }

    fn git_email(&self) -> String {
        self.env_var("GITLAB_USER_EMAIL")
    }

    fn git_token(&self) -> String {
        self.env_var_or("SEMVER_CI_TOKEN", &self.env_var("CI_JOB_TOKEN"))
    }

    fn create_release(&self, release: &Release) {
        println!(
            "{}, {}, {}, {}",
            release.name, release.description, release.tag_name, release.tag_message
        )
    }
}
