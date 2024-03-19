use crate::git_service;
use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use clap::Args;
use regex::Regex;

const DEFAULT_SEMANTIC_VERSION_TAG: &str = "v0.0.0";
const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
const RELEASE_CANDIDATE_PATTERN: &str = r"^(release|hotfix)/.*$";
const SEMANTIC_VERSION_TAG_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    #[arg(short, long, env, default_value = "minor")]
    scope: String,
}

pub(crate) fn run(args: VersionCommandArgs) {
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipelines::PipelineInfo::new(pipeline);

    let tag_names = git_service::tag_names(
        &pipeline_info.target_path,
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    );
    let last_tag = git_service::last_tag_by_pattern(
        tag_names,
        SEMANTIC_VERSION_TAG_PATTERN,
        DEFAULT_SEMANTIC_VERSION_TAG,
    );
    let version = version(args.scope, last_tag);

    let metadata = metadata(pipeline_info.branch_name, pipeline_info.short_commit_sha);

    println!("{}{}", version, metadata)
}

fn version(scope: String, last_tag: String) -> String {
    let mut semantic_version = SemanticVersion::from_string(last_tag.clone())
        .unwrap_or_else(|e| panic!("{}: {}", e, last_tag));

    semantic_version.increase_by_scope(scope).to_string(true)
}

fn metadata(branch_name: String, short_commit_sha: String) -> String {
    let dev_regex = Regex::new(DEV_PATTERN).unwrap_or_else(|e| panic!("{}", e));
    let release_candidate_regex =
        Regex::new(RELEASE_CANDIDATE_PATTERN).unwrap_or_else(|e| panic!("{}", e));

    if dev_regex.is_match(&branch_name) {
        format!("-dev.{}", short_commit_sha)
    } else if release_candidate_regex.is_match(&branch_name) {
        // TODO: Find "^v?(\d+\.\d+\.\d+)-rc\.\d+$" pattern tag and increase "rc\.\d+" number
        format!("-rc.{}", short_commit_sha)
    } else {
        "".to_string()
    }
}
