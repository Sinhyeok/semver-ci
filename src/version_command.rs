use crate::git_repo::GitRepo;
use crate::github_action::GithubAction;
use crate::gitlab_ci::GitlabCI;
use crate::pipeline::Pipeline;
use crate::semantic_version::SemanticVersion;
use crate::{git_service, VersionArgs};
use std::env;

const DEFAULT_SEMANTIC_VERSION_TAG: &str = "v0.0.0";

pub(crate) fn run(args: &VersionArgs) {
    let scope = args.scope.clone();
    let repo = match git_service::open_repo() {
        Ok(repo) => repo,
        Err(e) => {
            panic!("Failed to open git repo: {}", e)
        }
    };
    let latest_tag = git_service::latest_semantic_version_tag(&repo)
        .unwrap_or_else(|| DEFAULT_SEMANTIC_VERSION_TAG.to_string());
    let version = version(scope, latest_tag);

    let (branch_name, short_commit_sha) = pipeline_info();
    let metadata = metadata(branch_name, short_commit_sha);

    println!("{}{}", version, metadata)
}

fn version(scope: String, latest_tag: String) -> String {
    let mut semantic_version = match SemanticVersion::from_string(latest_tag[1..].to_string()) {
        Ok(semantic_version) => semantic_version,
        Err(e) => {
            panic!("{}: {}", e, latest_tag)
        }
    };

    match scope.as_str() {
        "major" => semantic_version.increase_major(),
        "minor" => semantic_version.increase_minor(),
        "patch" => semantic_version.increase_patch(),
        _ => {
            panic!("Invalid scope: {}", scope)
        }
    }

    semantic_version.to_string(true)
}

fn metadata(branch_name: String, short_commit_sha: String) -> String {
    if branch_name == "develop" {
        format!("-dev.{}", short_commit_sha)
    } else {
        "".to_string()
    }
}

fn pipeline_info() -> (String, String) {
    if env::var("GITLAB_CI").map_or(false, |v| v == "true") {
        let gitlab_ci = GitlabCI {};
        (gitlab_ci.branch_name(), gitlab_ci.short_commit_sha())
    } else if env::var("GITHUB_ACTIONS").map_or(false, |v| v == "true") {
        let github_action = GithubAction {};
        (
            github_action.branch_name(),
            github_action.short_commit_sha(),
        )
    } else {
        let git_repo = GitRepo {};
        (git_repo.branch_name(), git_repo.short_commit_sha())
    }
}
