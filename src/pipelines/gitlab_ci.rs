use crate::git_service;
use crate::pipelines::Pipeline;
use crate::release::Release;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct GitlabCI;

pub const GITLAB_CI: &str = "GITLAB_CI";

impl Pipeline for GitlabCI {
    fn init(&self) {
        self.git_origin_pushurl(self.env_var("CI_PROJECT_URL"));
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

    fn create_release(&self, release: &Release) -> HashMap<String, Value> {
        let path = format!("projects/{}/releases", self.env_var("CI_PROJECT_ID"));
        let mut body = HashMap::new();
        body.insert("name", release.name.clone());
        body.insert("description", release.description.clone());
        body.insert("tag_name", release.tag_name.clone());
        body.insert("tag_message", release.tag_message.clone());
        body.insert("ref", self.env_var("CI_COMMIT_SHA"));

        self.api_v4_post(path, body)
    }
}

impl GitlabCI {
    fn git_origin_pushurl(&self, url: String) {
        git_service::set_config_value("remote.origin.pushurl", &format!("{}.git", url))
            .unwrap_or_else(|e| panic!("{}", e));
    }

    fn api_v4_post(&self, path: String, body: HashMap<&str, String>) -> HashMap<String, Value> {
        let url = format!("{}/{}", self.env_var("CI_API_V4_URL"), path);
        let client = reqwest::blocking::Client::new();

        let response = client
            .post(url)
            .header("PRIVATE-TOKEN", self.env_var("DEV_TOKEN"))
            .json(&body)
            .send()
            .unwrap_or_else(|e| panic!("{}", e));

        let status = response.status();
        if status.is_success() {
            response
                .json::<HashMap<String, Value>>()
                .unwrap_or_else(|e| panic!("{}", e))
        } else {
            let headers = response.headers().clone();
            let body = response.text().unwrap();
            panic!(
                "Status: {}\nHeaders:\n{:#?}\nBody:\n{}",
                status, headers, body
            )
        }
    }
}
