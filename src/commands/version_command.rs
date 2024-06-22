use crate::default_error::DefaultError;
use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use crate::{config, git_service};
use clap::Args;
use git2::string_array::StringArray;
use regex::Regex;
use std::error::Error;

const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
const RELEASE_CANDIDATE_PATTERN: &str = r"^(release|hotfix)/.*$";
const SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";
const SEMANTIC_VERSION_TAG_ALL_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+.*)$";

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    #[arg(short, long, env, default_value = "minor")]
    scope: String,
}

pub(crate) fn run(args: VersionCommandArgs) -> Result<(), Box<dyn Error>> {
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
    .map_err(|e| {
        Box::new(DefaultError {
            message: "Failed to retrieve tags".to_string(),
            source: Some(Box::new(e)),
        })
    })?;

    // Last official tag
    let mut last_official_tag = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN,
        SemanticVersion::default(),
    );
    let mut last_tag = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_ALL_PATTERN,
        SemanticVersion::default(),
    );

    let upcoming_version;
    let last_version;

    let prerelease_stage = prerelease_stage(&pipeline_info.branch_name);
    if args.scope == "release" || prerelease_stage.is_empty() {
        upcoming_version = last_tag.release().to_string(true);
        last_version = last_official_tag.to_string(true);
    } else {
        let upcoming_official_version = last_official_tag.increase_by_scope(args.scope);
        upcoming_version = prerelease_version(
            &tag_names,
            prerelease_stage.clone(),
            upcoming_official_version,
            pipeline_info.short_commit_sha,
        );

        last_version = last_tag.to_string(true);
    }

    println!("UPCOMING_VERSION={}", upcoming_version);
    println!("LAST_VERSION={}", last_version);

    Ok(())
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
    upcoming_official_version: SemanticVersion,
    commit_short_sha: String,
) -> String {
    let upcoming_official_version_string = upcoming_official_version.to_string(false);
    let mut upcoming_version = upcoming_official_version.clone();
    upcoming_version
        .prerelease_stage
        .clone_from(&prerelease_stage);

    let mut upcoming_prerelease_version = git_service::last_tag_by_pattern(
        tag_names,
        &format!(
            r"^v?{}-{}\.[0-9]+.*$",
            upcoming_official_version_string, prerelease_stage
        ),
        upcoming_version,
    )
    .increase_by_scope("prerelease".to_string());
    upcoming_prerelease_version.commit_short_sha = commit_short_sha;

    upcoming_prerelease_version.to_string(true)
}
