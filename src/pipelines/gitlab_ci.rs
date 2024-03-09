use crate::pipelines::Pipeline;
use crate::release::Release;
use crate::{git_service, http_service};
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
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
        let url = format!(
            "{}/projects/{}/releases",
            self.env_var("CI_API_V4_URL"),
            self.env_var("CI_PROJECT_ID")
        );

        let mut headers = HeaderMap::new();
        headers.insert("JOB-TOKEN", self.env_var("CI_JOB_TOKEN").parse().unwrap());

        let mut body = HashMap::new();
        body.insert("name", json!(release.name.clone()));
        body.insert("description", json!(release.description.clone()));
        body.insert("tag_name", json!(release.tag_name.clone()));
        body.insert("tag_message", json!(release.tag_message.clone()));
        body.insert("ref", json!(self.env_var("CI_COMMIT_SHA")));

        http_service::post(url, Some(headers), Some(body))
    }
}

impl GitlabCI {
    fn git_origin_pushurl(&self, url: String) {
        let name = "remote.origin.pushurl";
        let value = format!("{}.git", url);
        git_service::set_config_value(&self.target_path(), name, &value)
            .unwrap_or_else(|e| panic!("{}", e));
    }
}
