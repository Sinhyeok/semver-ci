use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use crate::{config, git_service};
use clap::Args;
use git2::string_array::StringArray;
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
    let pipeline_info = pipeline.info();

    let tag_names = git_service::tag_names(
        &config::clone_target_path(),
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    )
    .unwrap_or_else(|e| panic!("Failed to retrieve tags: {}", e));

    let last_tag = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_PATTERN,
        DEFAULT_SEMANTIC_VERSION_TAG,
    );

    let version =
        version(args.scope, last_tag.clone()).unwrap_or_else(|e| panic!("{}: {}", e, last_tag));

    let prerelease_stage = prerelease_stage(&pipeline_info.branch_name);
    let prerelease_version_number =
        prerelease_version_number(&tag_names, &version, &prerelease_stage)
            .unwrap_or_else(|e| panic!("{}", e));
    let metadata = metadata(
        &prerelease_stage,
        &prerelease_version_number,
        &pipeline_info.short_commit_sha,
    );

    println!("{}{}", version, metadata)
}

fn version(scope: String, last_tag: String) -> Result<String, String> {
    let mut semantic_version = SemanticVersion::from_string(last_tag)?;

    Ok(semantic_version.increase_by_scope(scope).to_string(true))
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

fn increase_str(s: &str) -> Option<String> {
    let num: u64 = s.parse().ok()?;
    let increased_num = num + 1;
    Some(increased_num.to_string())
}

fn prerelease_version_number(
    tag_names: &StringArray,
    version: &str,
    stage: &str,
) -> Result<String, String> {
    let version_string = &git_service::last_tag_by_pattern(
        tag_names,
        &format!(r"^{}-{}\.\d+", version, stage),
        &format!("{}-{}.0.nothing", version, stage),
    );
    println!("version_string: {}", version_string);

    let capture_pattern = &format!(r"-{}\.(\d+)", stage);

    let re = Regex::new(capture_pattern).unwrap();

    if let Some(captures) = re.captures(version_string) {
        match captures.get(1) {
            Some(matched) => {
                let version = matched.as_str();
                match increase_str(version) {
                    Some(number) => Ok(number),
                    None => Err(format!("Invalid pre-release version: {}", version)),
                }
            }
            None => Err(format!("Pre-release version not found: {:?}", captures)),
        }
    } else {
        Err(format!(
            "Failed to capture pre-release version: {}",
            capture_pattern
        ))
    }
}

fn metadata(stage: &str, prerelease_version_number: &str, short_commit_sha: &str) -> String {
    match stage {
        "dev" => {
            format!(
                "-{}.{}.{}",
                stage, prerelease_version_number, short_commit_sha
            )
        }
        "rc" => {
            format!("-{}.{}", stage, prerelease_version_number)
        }
        _ => "".to_string(),
    }
}
