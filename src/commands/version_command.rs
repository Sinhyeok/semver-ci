use crate::default_error::DefaultError;
use crate::pipelines;
use crate::semantic_version::SemanticVersion;
use crate::{config, git_service};
use clap::Args;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;

const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
const RELEASE_CANDIDATE_PATTERN: &str = r"^(release|hotfix)/.*$";
const SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";
const SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+-.+)$";

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    #[arg(short, long, env, default_value = "minor")]
    scope: String,

    /// Promote this exact prerelease tag (official version calculation only).
    #[arg(long, env, value_name = "TAG")]
    candidate: Option<String>,
}

pub(crate) fn run(args: VersionCommandArgs) -> Result<(), Box<dyn Error>> {
    // Pipeline
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();

    let prerelease_stage = prerelease_stage(&pipeline_info.branch_name);
    let is_official = args.scope == "release" || prerelease_stage.is_empty();
    if args.candidate.is_some() && !is_official {
        return Err(DefaultError {
            message: "--candidate requires official version calculation; use --scope release on this branch.".to_string(),
            source: None,
        }.into());
    }

    // Tag names
    let all_tag_names = git_service::tag_names(
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

    let tag_names = if is_official {
        // Explicit promotion still derives LAST_VERSION from target history,
        // but does not infer a candidate from ancestry.
        let official_pattern = Regex::new(SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN).unwrap();
        let selection_tags: Vec<_> = all_tag_names
            .iter()
            .filter(|tag| args.candidate.is_none() || official_pattern.is_match(tag))
            .cloned()
            .collect();
        git_service::reachable_tag_names(
            &config::clone_target_path(),
            &selection_tags,
            &pipeline.target_commit(),
        )?
    } else {
        all_tag_names.clone()
    };

    // Last official tag (restricted to the target's history for promotion).
    let mut last_official_tag = git_service::last_tag_by_pattern(
        &tag_names,
        SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN,
        Some(SemanticVersion::default()),
    )
    .unwrap();

    // For release (main, master)
    let (upcoming_version, last_version) = if is_official {
        let upcoming = if let Some(candidate) = args.candidate.as_deref() {
            explicit_official_version(candidate, &all_tag_names, &last_official_tag)?
        } else {
            upcoming_official_version(&tag_names, &last_official_tag)?
        };
        (upcoming, last_official_tag.to_string(true))
    // For pre-release (develop, feature/*, release/*, hotfix/*)
    } else {
        let upcoming_official_version = last_official_tag.increase_by_scope(args.scope);

        (
            upcoming_prerelease_version(
                &tag_names,
                prerelease_stage.clone(),
                upcoming_official_version.clone(),
                pipeline_info.short_commit_sha,
            ),
            last_prerelease_version(
                &tag_names,
                prerelease_stage,
                last_official_tag,
                upcoming_official_version.to_string(false),
            ),
        )
    };

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

fn upcoming_official_version(
    tag_names: &[String],
    last_official_version: &SemanticVersion,
) -> Result<String, Box<dyn Error>> {
    let pattern = Regex::new(SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN).unwrap();
    let mut candidates: BTreeMap<(u64, u64, u64), Vec<&str>> = BTreeMap::new();
    for tag_name in tag_names {
        if !pattern.is_match(tag_name) {
            continue;
        }
        let Ok(mut version) = SemanticVersion::from_string(tag_name.clone()) else {
            continue;
        };
        let version = version.release();
        if version.cmp(last_official_version) == Ordering::Greater {
            candidates
                .entry((version.major, version.minor, version.patch))
                .or_default()
                .push(tag_name);
        }
    }

    if candidates.len() > 1 {
        let tags = candidates
            .values()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(DefaultError {
            message: format!(
                "Ambiguous official release: reachable prerelease tags target multiple versions: {tags}. Select one with --candidate <tag>."
            ),
            source: None,
        }.into());
    }
    if let Some((major, minor, patch)) = candidates.keys().next() {
        return Ok(format!("v{major}.{minor}.{patch}"));
    }

    log::warn!(
        "No newer reachable pre-release after last official tag ({}). Fallback to minor bump.",
        last_official_version.to_string(true)
    );
    Ok(last_official_version
        .clone()
        .increase_by_scope("minor".to_string())
        .to_string(true))
}

fn explicit_official_version(
    candidate: &str,
    all_tag_names: &[String],
    last_official_version: &SemanticVersion,
) -> Result<String, Box<dyn Error>> {
    let invalid = |reason: String| -> Box<dyn Error> {
        DefaultError {
            message: format!("Invalid candidate '{candidate}': {reason}"),
            source: None,
        }
        .into()
    };
    if !all_tag_names.iter().any(|tag| tag == candidate) {
        return Err(invalid(
            "tag not found; fetch the exact tag before retrying".to_string(),
        ));
    }
    let mut version = SemanticVersion::from_string(candidate.to_string()).map_err(&invalid)?;
    // The legacy parser tolerates extra fields. Explicit selection must name
    // one of the supported dev/rc formats without discarding part of the tag.
    if version.prerelease_stage.is_empty()
        || version.to_string(candidate.starts_with('v')) != candidate
    {
        return Err(invalid(
            "expected a valid dev or rc prerelease tag".to_string(),
        ));
    }
    git_service::tag_commit_id(&config::clone_target_path(), candidate)
        .map_err(|error| invalid(format!("tag must point to a commit: {error}")))?;
    let official = version.release();
    if official.cmp(last_official_version) != Ordering::Greater {
        return Err(invalid(format!(
            "version must be newer than the target's last official version ({})",
            last_official_version.to_string(true)
        )));
    }
    let official_pattern = Regex::new(SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN).unwrap();
    for tag in all_tag_names {
        if official_pattern.is_match(tag)
            && SemanticVersion::from_string(tag.clone())
                .is_ok_and(|version| version.cmp(&official) == Ordering::Equal)
        {
            return Err(invalid(format!(
                "official version already exists as tag '{tag}'"
            )));
        }
    }
    Ok(official.to_string(true))
}

fn upcoming_prerelease_version(
    tag_names: &[String],
    prerelease_stage: String,
    mut upcoming_official_version: SemanticVersion,
    commit_short_sha: String,
) -> String {
    let upcoming_official_version_string = upcoming_official_version.to_string(false);
    upcoming_official_version
        .prerelease_stage
        .clone_from(&prerelease_stage);

    let mut upcoming_prerelease_version = git_service::last_tag_by_pattern(
        tag_names,
        &format!(
            r"^v?{}-{}\.[0-9]+.*$",
            upcoming_official_version_string, prerelease_stage
        ),
        Some(upcoming_official_version),
    )
    .unwrap()
    .increase_by_scope("prerelease".to_string());
    upcoming_prerelease_version.commit_short_sha = commit_short_sha;

    upcoming_prerelease_version.to_string(true)
}

fn last_prerelease_version(
    tag_names: &[String],
    prerelease_stage: String,
    last_official_version: SemanticVersion,
    upcoming_official_version: String,
) -> String {
    git_service::last_tag_by_pattern(
        tag_names,
        &format!(
            r"^v?{}-{}\.[0-9]+.*$",
            upcoming_official_version, prerelease_stage
        ),
        Some(last_official_version),
    )
    .unwrap()
    .to_string(true)
}
