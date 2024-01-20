use crate::git_service;
use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use clap::Args;
use regex::Regex;

const DEFAULT_SEMANTIC_VERSION_TAG: &str = "v0.0.0";
const RELEASE_CANDIDATE_PATTERN: &str = r"^(release|hotfix)/.*$";

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    #[arg(short, long, default_value = "minor")]
    pub(crate) scope: String,
}

pub(crate) fn run(args: VersionCommandArgs) {
    let last_tag = git_service::last_semantic_version_tag()
        .unwrap_or(DEFAULT_SEMANTIC_VERSION_TAG.to_string());
    let version = version(args.scope, last_tag);

    let pipeline_info = pipelines::pipeline_info();
    let metadata = metadata(pipeline_info.branch_name, pipeline_info.short_commit_sha);

    println!("{}{}", version, metadata)
}

fn version(scope: String, last_tag: String) -> String {
    let mut semantic_version = match SemanticVersion::from_string(last_tag[1..].to_string()) {
        Ok(semantic_version) => semantic_version,
        Err(e) => {
            panic!("{}: {}", e, last_tag)
        }
    };

    semantic_version.increase_by_scope(scope);
    semantic_version.to_string(true)
}

fn metadata(branch_name: String, short_commit_sha: String) -> String {
    let release_candidate_regex =
        Regex::new(RELEASE_CANDIDATE_PATTERN).unwrap_or_else(|e| panic!("{}", e));

    if branch_name == "develop" {
        format!("-dev.{}", short_commit_sha)
    } else if release_candidate_regex.is_match(&branch_name) {
        // TODO: Find "^v?(\d+\.\d+\.\d+)-rc\.\d+$" pattern tag and increase "rc\.\d+" number
        format!("-rc.{}", short_commit_sha)
    } else {
        "".to_string()
    }
}
