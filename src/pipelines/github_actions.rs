use crate::git_service;
use crate::pipelines::Pipeline;
use std::env;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn init(&self) {
        // Clone repo
        let github_server_url = env::var("GITHUB_SERVER_URL")
            .unwrap_or_else(|e| panic!("{}: \"GITHUB_SERVER_URL\"", e));
        let github_repository = env::var("GITHUB_REPOSITORY")
            .unwrap_or_else(|e| panic!("{}: \"GITHUB_REPOSITORY\"", e));
        let repo_url = format!("{}/{}.git", github_server_url, github_repository);
        let target_path = env::var("CLONE_TARGET_PATH").unwrap_or(".".to_string());
        git_service::clone(
            &repo_url,
            &target_path,
            &self.git_username(),
            &self.git_token(),
        )
        .unwrap_or_else(|e| panic!("{}", e));

        // Git config: "safe.directory=."
        git_service::set_global_config_value("safe.directory", ".").unwrap();
    }

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

    fn git_email(&self) -> String {
        "41898282+github-actions[bot]@users.noreply.github.com".to_string()
    }

    fn git_token(&self) -> String {
        env::var("GITHUB_TOKEN").unwrap_or_else(|e| panic!("{}: \"GITHUB_TOKEN\"", e))
    }
}
