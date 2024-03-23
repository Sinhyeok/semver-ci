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
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();

    let tag_names = git_service::tag_names(
        &config::clone_target_path(),
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    )
    .unwrap_or_else(|e| panic!("Failed to retrieve tags: {}", e));

    let mut last_semantic_version = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_PATTERN,
        SemanticVersion {
            major: 0,
            minor: 0,
            patch: 0,
            prerelease_stage: "".to_string(),
            prerelease_number: 0,
        },
    );
    last_semantic_version.increase_by_scope(args.scope);

    let prerelease_stage = prerelease_stage(&pipeline_info.branch_name);
    let version = if prerelease_stage.is_empty() {
        last_semantic_version.to_string(true)
    } else {
        last_semantic_version.prerelease_stage = prerelease_stage.clone();

        prerelease_version(
            &tag_names,
            prerelease_stage,
            last_semantic_version,
            pipeline_info.short_commit_sha,
        )
    };

    println!("{}", version)
}

fn prerelease_stage(branch_name: &str) -> String {
    let dev_regex = Regex::new(DEV_PATTERN).unwrap_or_else(|e| panic!("{}", e));
    let release_candidate_regex =
        Regex::new(RELEASE_CANDIDATE_PATTERN).unwrap_or_else(|e| panic!("{}", e));

    if dev_regex.is_match(branch_name) {
        "dev".to_string()
    } else if release_candidate_regex.is_match(branch_name) {
        "rc".to_string()
    } else {
        "".to_string()
    }
}

fn prerelease_version(
    tag_names: &StringArray,
    prerelease_stage: String,
    last_semantic_version: SemanticVersion,
    short_commit_sha: String,
) -> String {
    let mut last_prerelease_semantic_version = git_service::last_tag_by_pattern(
        tag_names,
        &format!(
            r"^v?([0-9]+\.[0-9]+\.[0-9]+)-{}\.[0-9]+($|\.)",
            prerelease_stage
        ),
        last_semantic_version,
    );

    let prerelease_version = last_prerelease_semantic_version
        .increase_by_scope("prerelease".to_string())
        .to_string(true);

    if prerelease_stage == "dev" {
        format!("{}.{}", prerelease_version, short_commit_sha)
    } else {
        prerelease_version
    }
}
