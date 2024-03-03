use crate::pipelines::Pipeline;
use crate::release::Release;
use crate::{git_service, http_service};
use git2::Repository;
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn init(&self) {
        let target_path = self.env_var_or("CLONE_TARGET_PATH", ".");

        // Git config: "safe.directory=."
        Self::add_safe_directory(&target_path);

        // Clone
        if Repository::open(&target_path).is_err() {
            self.clone(&target_path);
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

    fn create_release(&self, release: &Release) -> HashMap<String, Value> {
        let url = format!(
            "{}/repos/{}/releases",
            self.env_var("GITHUB_API_URL"),
            self.env_var("GITHUB_REPOSITORY")
        );

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "Semver-CI".parse().unwrap());
        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.git_token()).parse().unwrap(),
        );

        let mut body = HashMap::new();
        body.insert("name", json!(release.name.clone()));
        body.insert("body", json!(release.description.clone()));
        body.insert("tag_name", json!(release.tag_name.clone()));
        body.insert("target_commitish", json!(self.env_var("GITHUB_SHA")));
        body.insert(
            "generate_release_notes",
            json!(release.generate_release_notes),
        );

        http_service::post(url, Some(headers), Some(body))
    }
}

impl GithubActions {
    fn add_safe_directory(path: &str) {
        git_service::set_global_config_value("safe.directory", path).unwrap();
    }

    fn clone(&self, target_path: &str) {
        // Clone repo
        let repo_url = format!(
            "{}/{}.git",
            self.env_var("GITHUB_SERVER_URL"),
            self.env_var("GITHUB_REPOSITORY")
        );
        let repo = git_service::clone(
            &repo_url,
            target_path,
            &self.git_username(),
            &self.git_token(),
            20,
        )
        .unwrap_or_else(|e| panic!("{}", e));

        // Fetch GITHUB_REF
        let github_ref = self.env_var("GITHUB_REF");
        let refspec = format!("{}:{}", github_ref, github_ref);
        git_service::fetch_refs(&repo, &self.git_username(), &self.git_token(), &[&refspec])
            .unwrap_or_else(|e| panic!("Failed to fetch GITHUB_REF: {}", e));

        // Checkout GITHUB_REF
        git_service::checkout(&repo, &github_ref).unwrap_or_else(|e| panic!("{}", e));
    }
}
