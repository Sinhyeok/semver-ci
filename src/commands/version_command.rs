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

type OfficialVersionCandidates<'a> = BTreeMap<(u64, u64, u64), Vec<&'a str>>;

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
    let candidates = collect_official_candidates(tag_names, last_official_version);
    match select_official_candidate(&candidates)? {
        Some(version) => Ok(version),
        None => Ok(minor_fallback_version(last_official_version)),
    }
}

fn collect_official_candidates<'a>(
    tag_names: &'a [String],
    last_official_version: &SemanticVersion,
) -> OfficialVersionCandidates<'a> {
    let pattern = Regex::new(SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN).unwrap();
    let mut candidates = OfficialVersionCandidates::new();
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
    candidates
}

fn select_official_candidate(
    candidates: &OfficialVersionCandidates<'_>,
) -> Result<Option<String>, Box<dyn Error>> {
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
    Ok(candidates
        .keys()
        .next()
        .map(|(major, minor, patch)| format!("v{major}.{minor}.{patch}")))
}

fn minor_fallback_version(last_official_version: &SemanticVersion) -> String {
    log::warn!(
        "No newer reachable pre-release after last official tag ({}). Fallback to minor bump.",
        last_official_version.to_string(true)
    );
    last_official_version
        .clone()
        .increase_by_scope("minor".to_string())
        .to_string(true)
}

fn explicit_official_version(
    candidate: &str,
    all_tag_names: &[String],
    last_official_version: &SemanticVersion,
) -> Result<String, Box<dyn Error>> {
    validate_candidate_tag_exists(candidate, all_tag_names)?;
    let mut version = parse_candidate_prerelease(candidate)?;
    validate_candidate_tag_commit(candidate)?;

    let official = version.release();
    validate_candidate_version_increase(candidate, &official, last_official_version)?;
    validate_candidate_not_released(candidate, &official, all_tag_names)?;

    Ok(official.to_string(true))
}

fn invalid_candidate(candidate: &str, reason: impl Into<String>) -> Box<dyn Error> {
    DefaultError {
        message: format!("Invalid candidate '{candidate}': {}", reason.into()),
        source: None,
    }
    .into()
}

fn validate_candidate_tag_exists(
    candidate: &str,
    all_tag_names: &[String],
) -> Result<(), Box<dyn Error>> {
    if !all_tag_names.iter().any(|tag| tag == candidate) {
        return Err(invalid_candidate(
            candidate,
            "tag not found; fetch the exact tag before retrying",
        ));
    }
    Ok(())
}

fn parse_candidate_prerelease(candidate: &str) -> Result<SemanticVersion, Box<dyn Error>> {
    let version = SemanticVersion::from_string(candidate.to_string())
        .map_err(|reason| invalid_candidate(candidate, reason))?;
    // The legacy parser tolerates extra fields. Explicit selection must name
    // one of the supported dev/rc formats without discarding part of the tag.
    if version.prerelease_stage.is_empty()
        || version.to_string(candidate.starts_with('v')) != candidate
    {
        return Err(invalid_candidate(
            candidate,
            "expected a valid dev or rc prerelease tag",
        ));
    }
    Ok(version)
}

fn validate_candidate_tag_commit(candidate: &str) -> Result<(), Box<dyn Error>> {
    git_service::tag_commit_id(&config::clone_target_path(), candidate).map_err(|error| {
        invalid_candidate(candidate, format!("tag must point to a commit: {error}"))
    })?;
    Ok(())
}

fn validate_candidate_version_increase(
    candidate: &str,
    official: &SemanticVersion,
    last_official_version: &SemanticVersion,
) -> Result<(), Box<dyn Error>> {
    if official.cmp(last_official_version) != Ordering::Greater {
        return Err(invalid_candidate(
            candidate,
            format!(
                "version must be newer than the target's last official version ({})",
                last_official_version.to_string(true)
            ),
        ));
    }
    Ok(())
}

fn validate_candidate_not_released(
    candidate: &str,
    official: &SemanticVersion,
    all_tag_names: &[String],
) -> Result<(), Box<dyn Error>> {
    let official_pattern = Regex::new(SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN).unwrap();
    for tag in all_tag_names {
        if official_pattern.is_match(tag)
            && SemanticVersion::from_string(tag.clone())
                .is_ok_and(|version| version.cmp(official) == Ordering::Equal)
        {
            return Err(invalid_candidate(
                candidate,
                format!("official version already exists as tag '{tag}'"),
            ));
        }
    }
    Ok(())
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
