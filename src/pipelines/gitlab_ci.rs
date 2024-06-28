use crate::pipelines::Pipeline;
use crate::release::Release;
use crate::{config, git_service, http_service};
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;

pub(crate) struct GitlabCI;

pub const GITLAB_CI: &str = "GITLAB_CI";
const IGNORE_CHANGE_PREFIXES: [&str; 5] = ["refactor:", "style:", "test:", "chore:", "Merge "];

impl Pipeline for GitlabCI {
    fn init(&self) {
        self.git_origin_pushurl(config::env_var("CI_PROJECT_URL"));
    }

    fn name(&self) -> String {
        "GitLab CI".to_string()
    }

    fn branch_name(&self) -> String {
        config::env_var("CI_COMMIT_REF_NAME")
    }

    fn short_commit_sha(&self) -> String {
        config::env_var("CI_COMMIT_SHORT_SHA")
    }

    fn git_username(&self) -> String {
        "gitlab-ci-token".to_string()
    }

    fn git_email(&self) -> String {
        config::env_var("GITLAB_USER_EMAIL")
    }

    fn git_token(&self) -> String {
        config::env_var_or("SEMVER_CI_TOKEN", &config::env_var("CI_JOB_TOKEN"))
    }

    fn create_release(&self, release: &Release) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let url = format!(
            "{}/projects/{}/releases",
            config::env_var("CI_API_V4_URL"),
            config::env_var("CI_PROJECT_ID")
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "JOB-TOKEN",
            config::env_var("CI_JOB_TOKEN").parse().unwrap(),
        );

        let description = self.release_notes(
            release.description.clone(),
            release.generate_release_notes,
            &release.previous_tag,
            &config::env_var("CI_COMMIT_SHA"),
        );

        let mut body = HashMap::new();
        body.insert("name", json!(release.name.clone()));
        body.insert("description", json!(description));
        body.insert("tag_name", json!(release.tag_name.clone()));
        body.insert("tag_message", json!(release.tag_message.clone()));
        body.insert("ref", json!(config::env_var("CI_COMMIT_SHA")));

        http_service::post(url, Some(headers), Some(body))
    }
}

impl GitlabCI {
    fn git_origin_pushurl(&self, url: String) {
        let name = "remote.origin.pushurl";
        let value = format!("{}.git", url);
        git_service::set_config_value(&config::clone_target_path(), name, &value)
            .unwrap_or_else(|e| panic!("{}", e));
    }

    fn release_notes(&self, prepend: String, auto_generate: bool, from: &str, to: &str) -> String {
        let mut notes = prepend.clone();

        if auto_generate {
            if from == "v0.0.0" {
                notes += &self.commits(to);
            } else {
                notes += &self.compare(from, to).unwrap_or_else(|e| {
                    println!("{}", e);
                    self.web_compare_url(from, to)
                })
            }
        }

        notes
    }

    fn commits(&self, ref_name: &str) -> String {
        let commits = format!(
            "{}/-/commits/{}",
            config::env_var("CI_PROJECT_URL"),
            ref_name
        );

        format!(r#"Full Changelog: {}"#, commits)
    }

    fn web_compare_url(&self, from: &str, to: &str) -> String {
        format!(
            r#"# What's Changed
{}/-/compare/{}...{}"#,
            config::env_var("CI_PROJECT_URL"),
            from,
            to
        )
    }

    fn compare(&self, from: &str, to: &str) -> Result<String, Box<dyn Error>> {
        let url = format!(
            "{}/projects/{}/repository/compare",
            config::env_var("CI_API_V4_URL"),
            config::env_var("CI_PROJECT_ID")
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "JOB-TOKEN",
            config::env_var("CI_JOB_TOKEN").parse().unwrap(),
        );

        let mut query = HashMap::new();
        query.insert("from", from);
        query.insert("to", to);

        let parsed = http_service::get(url, Some(headers), Some(query))?;
        let commits = self.collect_commits(&parsed);
        let empty_string_value = Value::String("".to_string());
        let full_diff = parsed
            .get("web_url")
            .unwrap_or(&empty_string_value)
            .as_str()
            .unwrap_or("");

        Ok(format!(
            r#"## What's Changed
{}

Full Changelog: {}"#,
            commits, full_diff
        ))
    }

    fn collect_commits(&self, compare_response: &HashMap<String, Value>) -> String {
        let empty_string_value = Value::String("".to_string());

        compare_response
            .get("commits")
            .unwrap_or(&Value::Array(vec![]))
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|object| {
                let message = object
                    .get("message")
                    .unwrap_or(&empty_string_value)
                    .as_str()
                    .unwrap_or("");
                let web_url = object
                    .get("web_url")
                    .unwrap_or(&empty_string_value)
                    .as_str()
                    .unwrap_or("");
                if (message.is_empty() && web_url.is_empty())
                    || IGNORE_CHANGE_PREFIXES
                        .iter()
                        .any(|&prefix| message.starts_with(prefix))
                {
                    None
                } else {
                    Some(format!("* [{}]({})", message, web_url))
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
