use crate::git_service;
use crate::pipelines::Pipeline;
use crate::release::Release;
use git2::Repository;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn init(&self) {
        // Git config: "safe.directory=."
        git_service::set_global_config_value("safe.directory", ".").unwrap();

        if Repository::open(".").is_err() {
            self.clone()
        }
    }

    fn name(&self) -> String {
        "Github Actions".to_string()
    }

    fn branch_name(&self) -> String {
        self.env_var("GITHUB_REF_NAME")
    }

    fn short_commit_sha(&self) -> String {
        let commit_sha = self.env_var("GITHUB_SHA");
        commit_sha[0..8].to_owned()
    }

    fn git_username(&self) -> String {
        self.env_var("GITHUB_ACTOR")
    }

    fn git_email(&self) -> String {
        "41898282+github-actions[bot]@users.noreply.github.com".to_string()
    }

    fn git_token(&self) -> String {
        self.env_var("GITHUB_TOKEN")
    }

    fn create_release(&self, release: &Release) {
        println!(
            "{}, {}, {}, {}",
            release.name, release.description, release.tag_name, release.tag_message
        )
    }
}

impl GithubActions {
    fn clone(&self) {
        // Clone repo
        let github_server_url = self.env_var("GITHUB_SERVER_URL");
        let github_repository = self.env_var("GITHUB_REPOSITORY");
        let repo_url = format!("{}/{}.git", github_server_url, github_repository);
        let target_path = self.env_var_or("CLONE_TARGET_PATH", ".");
        let repo = git_service::clone(
            &repo_url,
            &target_path,
            &self.git_username(),
            &self.git_token(),
            20,
        )
        .unwrap_or_else(|e| panic!("{}", e));

        // Checkout GITHUB_REF
        let github_ref = self.env_var("GITHUB_REF");
        git_service::checkout(&repo, &github_ref).unwrap_or_else(|e| panic!("{}", e));
    }
}
