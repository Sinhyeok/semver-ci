use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use crate::{config, git_service};
use clap::Args;
use git2::string_array::StringArray;
use regex::Regex;

const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
const RELEASE_CANDIDATE_PATTERN: &str = r"^(release|hotfix)/.*$";
const SEMANTIC_VERSION_TAG_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    #[arg(short, long, env, default_value = "minor")]
    scope: String,
}

pub(crate) fn run(args: VersionCommandArgs) {
    // Pipeline
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();

    // Tag names
    let tag_names = git_service::tag_names(
        &config::clone_target_path(),
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    )
    .unwrap_or_else(|e| panic!("Failed to retrieve tags: {}", e));

    // Upcoming version
    let upcoming_version = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_PATTERN,
        SemanticVersion {
            major: 0,
            minor: 0,
            patch: 0,
            prerelease_stage: "".to_string(),
            prerelease_number: 0,
        },
    )
    .increase_by_scope(args.scope);

    // Pre-release stage
    let prerelease_stage = prerelease_stage(&pipeline_info.branch_name);

    // Version
    let version = if prerelease_stage.is_empty() {
        upcoming_version.to_string(true)
    } else {
        prerelease_version(
            &tag_names,
            prerelease_stage,
            upcoming_version,
            pipeline_info.short_commit_sha,
        )
    };

    println!("{}", version)
}

fn prerelease_stage(branch_name: &str) -> String {
    let dev_regex = Regex::new(DEV_PATTERN).unwrap_or_else(|e| panic!("{}", e));
    let release_candidate_regex =
        Regex::new(RELEASE_CANDIDATE_PATTERN).unwrap_or_else(|e| panic!("{}", e));

    let stage = if dev_regex.is_match(branch_name) {
        "dev"
    } else if release_candidate_regex.is_match(branch_name) {
        "rc"
    } else {
        ""
    };

    stage.to_string()
}

fn prerelease_version(
    tag_names: &StringArray,
    prerelease_stage: String,
    mut upcoming_version: SemanticVersion,
    short_commit_sha: String,
) -> String {
    upcoming_version.prerelease_stage = prerelease_stage.clone();
    let upcoming_prerelease_version = git_service::last_tag_by_pattern(
        tag_names,
        &format!(
            r"^v?([0-9]+\.[0-9]+\.[0-9]+)-{}\.[0-9]+($|\.)",
            prerelease_stage
        ),
        upcoming_version,
    )
    .increase_by_scope("prerelease".to_string());

    let upcoming_prerelease_string = upcoming_prerelease_version.to_string(true);
    if prerelease_stage == "dev" {
        format!("{}.{}", upcoming_prerelease_string, short_commit_sha)
    } else {
        upcoming_prerelease_string
    }
}
