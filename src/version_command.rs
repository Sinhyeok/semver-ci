use crate::pipeline;
use crate::pipeline::{Pipeline, PipelineType};
use crate::semantic_version::SemanticVersion;
use crate::{git_service, VersionArgs};
use regex::Regex;

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
        .unwrap_or(DEFAULT_SEMANTIC_VERSION_TAG.to_string());
    let version = version(scope, latest_tag);

    let pipeline_type = pipeline::pipeline_type();
    let metadata = match pipeline_type {
        PipelineType::GithubActions(p) => metadata(p.branch_name(), p.short_commit_sha()),
        PipelineType::GitlabCI(p) => metadata(p.branch_name(), p.short_commit_sha()),
        PipelineType::GitRepo(p) => metadata(p.branch_name(), p.short_commit_sha()),
    };

    println!("{}{}", version, metadata)
}

fn version(scope: String, latest_tag: String) -> String {
    let mut semantic_version = match SemanticVersion::from_string(latest_tag[1..].to_string()) {
        Ok(semantic_version) => semantic_version,
        Err(e) => {
            panic!("{}: {}", e, latest_tag)
        }
    };

    semantic_version.increase_by_scope(scope);
    semantic_version.to_string(true)
}

fn metadata(branch_name: String, short_commit_sha: String) -> String {
    let release_candidate_pattern =
        Regex::new(r"^(release|hotfix)/.*$").unwrap_or_else(|e| panic!("{}", e));

    if branch_name == "develop" {
        format!("-dev.{}", short_commit_sha)
    } else if release_candidate_pattern.is_match(&branch_name) {
        // TODO: Find "^v?(\d+\.\d+\.\d+)-rc\.\d+$" pattern tag and increase "rc\.\d+" number
        format!("-rc.{}", short_commit_sha)
    } else {
        "".to_string()
    }
}
