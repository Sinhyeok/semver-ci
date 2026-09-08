use crate::default_error::{DefaultError, Result, ResultExt};
use crate::error_messages as messages;
use crate::pipelines::Pipeline;
use crate::release::Release;
use crate::{config, git_service, http_service};
use git2::Repository;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) struct GithubActions;

pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

impl Pipeline for GithubActions {
    fn init(&self) -> Result<()> {
        // Git config: "safe.directory=."
        self.add_safe_directory()?;

        // Clone
        let path = config::clone_target_path()?;
        match Repository::open(&path) {
            Ok(_) => {}
            Err(error) if error.code() == git2::ErrorCode::NotFound => self.clone()?,
            Err(error) => {
                return Err(DefaultError::new(messages::open_repository(&path)).with_source(error))
            }
        }
        Ok(())
    }

    fn name(&self) -> String {
        "Github Actions".to_string()
    }

    fn branch_name(&self) -> Result<String> {
        config::env_var("GITHUB_REF_NAME")
    }

    fn short_commit_sha(&self) -> Result<String> {
        let commit_sha = config::env_var("GITHUB_SHA")?;
        if commit_sha.len() < 8 || !commit_sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(DefaultError::new(messages::invalid_commit_sha(
                "GITHUB_SHA",
            )));
        }
        Ok(commit_sha[..8].to_owned())
    }

    fn target_commit(&self) -> Result<String> {
        config::env_var("GITHUB_SHA")
    }

    fn git_username(&self) -> Result<String> {
        config::env_var("GITHUB_ACTOR")
    }

    fn git_email(&self) -> Result<String> {
        Ok("41898282+github-actions[bot]@users.noreply.github.com".to_string())
    }

    fn git_token(&self) -> Result<String> {
        config::env_var("GITHUB_TOKEN")
    }

    fn create_release(&self, release: &Release) -> Result<HashMap<String, Value>> {
        let url = format!(
            "{}/repos/{}/releases",
            config::env_var("GITHUB_API_URL")?,
            config::env_var("GITHUB_REPOSITORY")?
        );

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", HeaderValue::from_static("Semver-CI"));
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.git_token()?))
                .context(messages::invalid_header("Authorization"))?,
        );

        let mut body = HashMap::new();
        body.insert("name", json!(release.name.clone()));
        body.insert("body", json!(release.description.clone()));
        body.insert("tag_name", json!(release.tag_name.clone()));
        body.insert("prerelease", json!(release.is_prerelease()));
        body.insert("target_commitish", json!(config::env_var("GITHUB_SHA")?));
        body.insert(
            "generate_release_notes",
            json!(release.generate_release_notes),
        );

        http_service::post(url, Some(headers), Some(body))
    }
}

impl GithubActions {
    fn add_safe_directory(&self) -> Result<()> {
        git_service::set_global_config_value("safe.directory", &config::clone_target_path()?)
    }

    fn clone(&self) -> Result<()> {
        // Clone repo
        let repo_url = format!(
            "{}/{}.git",
            config::env_var("GITHUB_SERVER_URL")?,
            config::env_var("GITHUB_REPOSITORY")?
        );
        let repo = git_service::clone(
            &repo_url,
            &config::clone_target_path()?,
            &self.git_username()?,
            &self.git_token()?,
            0,
        )?;

        // Fetch GITHUB_REF
        let github_ref = config::env_var("GITHUB_REF")?;
        let refspec = format!("{}:{}", github_ref, github_ref);
        git_service::fetch_refs(
            &repo,
            &self.git_username()?,
            &self.git_token()?,
            &[&refspec],
        )?;

        // A branch may advance after this workflow starts; use the event commit.
        git_service::checkout(&repo, &self.target_commit()?)
    }
}
