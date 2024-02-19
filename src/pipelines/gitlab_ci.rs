use crate::git_service;
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

    fn git_email(&self) -> String {
        env::var("GITLAB_USER_EMAIL").unwrap_or_else(|e| panic!("{}: \"GITLAB_USER_EMAIL\"", e))
    }

    fn git_token(&self) -> String {
        match env::var("CI_PROJECT_URL") {
            Ok(project_url) => {
                git_service::set_config_value(
                    "remote.origin.pushurl",
                    &format!("{}.git", project_url),
                )
                .unwrap_or_else(|e| panic!("{}", e));
            }
            Err(e) => panic!("{}: CI_PROJECT_URL", e),
        }

        env::var("SEMVER_CI_TOKEN").unwrap_or(
            env::var("CI_JOB_TOKEN").unwrap_or_else(|e| panic!("{}: \"CI_JOB_TOKEN\"", e)),
        )
    }
}
