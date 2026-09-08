use crate::branch_rules::{Scope, Stage};
use crate::default_error::{DefaultError, Result, ResultExt};
use crate::error_messages as messages;
use crate::git_service;
use crate::release_target::ReleaseTarget;
use crate::semantic_version::SemanticVersion;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};

const SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";
const SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+-.+)$";
const PRERELEASE_SELECTION_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)-(dev|rc)\.[0-9]+.*$";

#[derive(PartialEq, Eq)]
enum TagKind {
    Official,
    Prerelease,
    Other,
}

struct VersionTag {
    name: String,
    version: SemanticVersion,
    kind: TagKind,
}

/// Inputs for version selection, independent of CLI arguments and pipeline types.
pub(crate) struct VersionRequest<'a> {
    pub repo_path: &'a str,
    pub target_commit: &'a str,
    pub short_commit_sha: &'a str,
    pub scope: Scope,
    pub stage: Stage,
    pub candidate: Option<&'a str>,
    pub target: Option<&'a ReleaseTarget>,
    pub tag_names: &'a [String],
}

pub(crate) struct VersionResult {
    pub upcoming_version: String,
    pub last_version: String,
}

pub(crate) fn calculate(request: VersionRequest<'_>) -> Result<VersionResult> {
    let all_tags = parse_version_tags(request.tag_names)?;
    let reachable_tags = select_reachable_tags(&request, &all_tags)?;
    let last_official = resolve_base(&reachable_tags, request.target)?;

    if let Some(target) = request.target {
        validate_target(target, &last_official, &all_tags)?;
    }

    let result = match request.stage {
        Stage::Stable => calculate_stable(&request, &reachable_tags, &all_tags, &last_official),
        Stage::Dev | Stage::Rc => calculate_prerelease(&request, &reachable_tags, &last_official),
    }?;
    validate_upcoming_version(&result.upcoming_version, request.tag_names)?;
    Ok(result)
}

fn validate_upcoming_version(upcoming_version: &str, tag_names: &[String]) -> Result<()> {
    let version = upcoming_version
        .strip_prefix('v')
        .unwrap_or(upcoming_version);
    // Compare full names: version ordering ignores dev SHAs, and parsing can
    // discard unsupported suffixes. Only the optional v prefix is equivalent.
    if let Some(tag) = tag_names
        .iter()
        .find(|tag| tag.strip_prefix('v').unwrap_or(tag) == version)
    {
        return Err(DefaultError::new(messages::upcoming_version_exists(
            upcoming_version,
            tag,
        )));
    }
    Ok(())
}

fn calculate_stable(
    request: &VersionRequest<'_>,
    reachable_tags: &[&VersionTag],
    all_tags: &[VersionTag],
    last_official: &SemanticVersion,
) -> Result<VersionResult> {
    let upcoming_version = match (request.candidate, request.scope) {
        (Some(candidate), _) => promote_explicit_candidate(
            request.repo_path,
            candidate,
            request.tag_names,
            all_tags,
            last_official,
        )?,
        (None, Scope::Release) => {
            promote_reachable_candidate(reachable_tags, last_official, request.target)?
        }
        (None, scope) => resolve_next_core(last_official, request.target, scope)?.to_string(true),
    };
    validate_result_target(request.target, &upcoming_version)?;

    Ok(VersionResult {
        upcoming_version,
        last_version: last_official.to_string(true),
    })
}

fn calculate_prerelease(
    request: &VersionRequest<'_>,
    reachable_tags: &[&VersionTag],
    last_official: &SemanticVersion,
) -> Result<VersionResult> {
    let stage = request.stage.as_str();
    let mut upcoming_core = resolve_next_core(last_official, request.target, request.scope)?;
    let core = upcoming_core.to_string(false);
    let pattern = Regex::new(PRERELEASE_SELECTION_PATTERN)
        .context(messages::invalid_regex(PRERELEASE_SELECTION_PATTERN))?;
    let latest_prerelease = latest_version(
        reachable_tags
            .iter()
            .filter(|tag| {
                pattern
                    .captures(&tag.name)
                    .is_some_and(|captures| &captures[1] == core.as_str() && &captures[2] == stage)
            })
            .map(|tag| &tag.version),
    );
    let last_version = latest_prerelease
        .as_ref()
        .unwrap_or(last_official)
        .to_string(true);

    upcoming_core.prerelease_stage = stage.to_string();
    let mut upcoming = latest_prerelease
        .unwrap_or(upcoming_core)
        .increase_by_scope("prerelease".to_string())?;
    upcoming.commit_short_sha = request.short_commit_sha.to_string();

    Ok(VersionResult {
        upcoming_version: upcoming.to_string(true),
        last_version,
    })
}

fn resolve_next_core(
    last_official: &SemanticVersion,
    target: Option<&ReleaseTarget>,
    scope: Scope,
) -> Result<SemanticVersion> {
    match target {
        Some(target) => Ok(target.version.clone()),
        None => last_official.increase_by_scope(scope.as_str().to_string()),
    }
}

fn validate_target(
    target: &ReleaseTarget,
    last_official: &SemanticVersion,
    all_tags: &[VersionTag],
) -> Result<()> {
    if target.version.cmp(last_official) != Ordering::Greater {
        return Err(DefaultError::new(messages::target_not_newer(
            &target.version.to_string(true),
            &last_official.to_string(true),
        )));
    }
    if let Some(tag) = find_published_official(all_tags, &target.version) {
        return Err(DefaultError::new(messages::target_released(
            &target.version.to_string(true),
            tag,
        )));
    }
    Ok(())
}

fn validate_result_target(target: Option<&ReleaseTarget>, upcoming_version: &str) -> Result<()> {
    let Some(target) = target else {
        return Ok(());
    };
    if upcoming_version != target.version.to_string(true) {
        return Err(DefaultError::new(messages::candidate_target_conflict(
            upcoming_version,
            &target.version.to_string(true),
        )));
    }
    Ok(())
}

fn resolve_base(
    reachable_tags: &[&VersionTag],
    target: Option<&ReleaseTarget>,
) -> Result<SemanticVersion> {
    let official_versions = reachable_tags
        .iter()
        .filter(|tag| tag.kind == TagKind::Official)
        .map(|tag| &tag.version);
    let Some(target) = target else {
        return Ok(latest_version(official_versions).unwrap_or_else(SemanticVersion::default));
    };

    // Only history with no official version may start from v0.0.0.
    let mut official_versions = official_versions.peekable();
    let initial_version = SemanticVersion::default();
    let fallback = official_versions
        .peek()
        .is_none()
        .then_some(&initial_version);
    latest_version(
        official_versions
            .chain(fallback)
            .filter(|version| target.includes_base(version)),
    )
    .ok_or_else(|| DefaultError::new(messages::no_target_base(&target.version.to_string(true))))
}

fn parse_version_tags(tag_names: &[String]) -> Result<Vec<VersionTag>> {
    let official_pattern = Regex::new(SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN).context(
        messages::invalid_regex(SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN),
    )?;
    let prerelease_pattern = Regex::new(SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN).context(
        messages::invalid_regex(SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN),
    )?;
    Ok(tag_names
        .iter()
        .filter_map(|name| {
            let version = SemanticVersion::from_string(name.clone()).ok()?;
            // Keep name classification separate from the permissive version parser.
            let kind = if official_pattern.is_match(name) {
                TagKind::Official
            } else if prerelease_pattern.is_match(name) {
                TagKind::Prerelease
            } else {
                TagKind::Other
            };
            Some(VersionTag {
                name: name.clone(),
                version,
                kind,
            })
        })
        .collect())
}

fn select_reachable_tags<'a>(
    request: &VersionRequest<'_>,
    all_tags: &'a [VersionTag],
) -> Result<Vec<&'a VersionTag>> {
    // Version inference uses the target's history for both official bases and
    // prereleases. Stable bumps and explicit promotion only need official bases.
    let include_prereleases = request.stage != Stage::Stable
        || (request.scope == Scope::Release && request.candidate.is_none());
    let selection_tags: Vec<_> = all_tags
        .iter()
        .filter(|tag| include_prereleases || tag.kind == TagKind::Official)
        .filter(|tag| match request.target {
            Some(_) if tag.kind == TagKind::Official => true,
            Some(target) => tag.version.release() == target.version,
            None => true,
        })
        .map(|tag| tag.name.clone())
        .collect();
    let reachable_names: HashSet<_> = git_service::reachable_tag_names(
        request.repo_path,
        &selection_tags,
        request.target_commit,
    )?
    .into_iter()
    .collect();
    Ok(all_tags
        .iter()
        .filter(|tag| reachable_names.contains(&tag.name))
        .collect())
}

fn promote_reachable_candidate(
    reachable_tags: &[&VersionTag],
    last_official_version: &SemanticVersion,
    target: Option<&ReleaseTarget>,
) -> Result<String> {
    match (
        select_official_candidate(reachable_tags, last_official_version)?,
        target,
    ) {
        (Some(version), _) => Ok(version),
        (None, Some(target)) => Err(DefaultError::new(messages::no_target_prerelease(
            &target.version.to_string(true),
            target.scope.as_str(),
        ))),
        (None, None) => {
            log::warn!(
                "{}",
                messages::minor_bump_fallback(&last_official_version.to_string(true))
            );
            Ok(last_official_version
                .increase_by_scope("minor".to_string())?
                .to_string(true))
        }
    }
}

fn select_official_candidate(
    reachable_tags: &[&VersionTag],
    last_official_version: &SemanticVersion,
) -> Result<Option<String>> {
    let mut candidates: BTreeMap<_, Vec<&str>> = BTreeMap::new();
    for tag in reachable_tags {
        if tag.kind != TagKind::Prerelease {
            continue;
        }
        let version = tag.version.release();
        if version.cmp(last_official_version) == Ordering::Greater {
            candidates
                .entry((version.major, version.minor, version.patch))
                .or_default()
                .push(tag.name.as_str());
        }
    }

    if candidates.len() > 1 {
        let tags = candidates
            .values()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(DefaultError::new(messages::ambiguous_release(&tags)));
    }
    Ok(candidates
        .keys()
        .next()
        .map(|(major, minor, patch)| format!("v{major}.{minor}.{patch}")))
}

fn promote_explicit_candidate(
    repo_path: &str,
    candidate: &str,
    all_tag_names: &[String],
    all_tags: &[VersionTag],
    last_official_version: &SemanticVersion,
) -> Result<String> {
    if !all_tag_names.iter().any(|tag| tag == candidate) {
        return Err(invalid_candidate(candidate, messages::CANDIDATE_NOT_FOUND));
    }

    let version = SemanticVersion::from_string(candidate.to_string())
        .context(messages::invalid_candidate(candidate))?;
    // The legacy parser tolerates extra fields. Explicit selection must name
    // one of the supported dev/rc formats without discarding part of the tag.
    if version.prerelease_stage.is_empty()
        || version.to_string(candidate.starts_with('v')) != candidate
    {
        return Err(invalid_candidate(candidate, messages::CANDIDATE_FORMAT));
    }

    git_service::tag_commit_id(repo_path, candidate)
        .context(messages::CANDIDATE_COMMIT)
        .context(messages::invalid_candidate(candidate))?;

    let official = version.release();
    if official.cmp(last_official_version) != Ordering::Greater {
        return Err(invalid_candidate(
            candidate,
            messages::candidate_not_newer(&last_official_version.to_string(true)),
        ));
    }
    if let Some(tag) = find_published_official(all_tags, &official) {
        return Err(invalid_candidate(
            candidate,
            messages::candidate_released(tag),
        ));
    }
    Ok(official.to_string(true))
}

fn invalid_candidate(candidate: &str, reason: impl Into<String>) -> DefaultError {
    DefaultError::new(messages::invalid_candidate(candidate)).with_source(DefaultError::new(reason))
}

fn find_published_official<'a>(
    all_tags: &'a [VersionTag],
    official: &SemanticVersion,
) -> Option<&'a str> {
    all_tags
        .iter()
        .find(|tag| tag.kind == TagKind::Official && tag.version == *official)
        .map(|tag| tag.name.as_str())
}

fn latest_version<'a>(
    versions: impl Iterator<Item = &'a SemanticVersion>,
) -> Option<SemanticVersion> {
    // Preserve the first tag when precedence is equal, including different dev SHAs.
    versions
        .reduce(|latest, version| if version > latest { version } else { latest })
        .cloned()
}
