use crate::errors::{messages, DefaultError, Result, ResultExt};
use crate::models::{ReleaseTarget, Scope, Stage};
use regex::Regex;

// Preserve the legacy scope regexes and their first-match precedence.
pub(crate) const MAJOR_PATTERN: &str = r"^release/[0-9]+.x.x$";
pub(crate) const MINOR_PATTERN: &str = r"^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$";
pub(crate) const PATCH_PATTERN: &str = r"^hotfix/[0-9]+.[0-9]+.[0-9]+$";
pub(crate) const STABLE_PATTERN: &str = r"^(main|master)$";
pub(crate) const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
pub(crate) const RC_PATTERN: &str = r"^(release|hotfix)/.*$";

pub(crate) struct ScopePatterns {
    pub major: String,
    pub minor: String,
    pub patch: String,
    pub release: String,
}

impl ScopePatterns {
    pub(crate) fn resolve(&self, branch: &str) -> Result<Scope> {
        matching_rule(
            branch,
            &[
                (Scope::Major, &self.major),
                (Scope::Minor, &self.minor),
                (Scope::Patch, &self.patch),
                (Scope::Release, &self.release),
            ],
        )?
        .ok_or_else(|| DefaultError::new(messages::unknown_branch(branch)))
    }
}

pub(crate) struct StagePatterns {
    pub dev: String,
    pub rc: String,
    pub stable: String,
}

impl StagePatterns {
    pub(crate) fn resolve(&self, branch: &str) -> Result<Stage> {
        matching_rule(
            branch,
            &[
                (Stage::Dev, &self.dev),
                (Stage::Rc, &self.rc),
                (Stage::Stable, &self.stable),
            ],
        )?
        .ok_or_else(|| DefaultError::new(messages::unknown_stage(branch)))
    }
}

fn matching_rule<T: Copy>(branch: &str, patterns: &[(T, &str)]) -> Result<Option<T>> {
    // Validate every configured pattern in the requested dimension before matching,
    // so an early match cannot hide a malformed later rule.
    let rules = patterns
        .iter()
        .map(|(value, pattern)| {
            Ok((
                *value,
                Regex::new(pattern).context(messages::invalid_regex(pattern))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(rules
        .into_iter()
        .find(|(_, regex)| regex.is_match(branch))
        .map(|(value, _)| value))
}

pub(crate) fn validate_stage(stage: Stage, has_candidate: bool) -> Result<()> {
    if has_candidate && stage != Stage::Stable {
        return Err(DefaultError::new(messages::CANDIDATE_STAGE));
    }
    Ok(())
}

pub(crate) fn validate_scope(scope: Scope, stage: Stage, has_candidate: bool) -> Result<()> {
    if has_candidate && scope != Scope::Release {
        return Err(DefaultError::new(messages::CANDIDATE_SCOPE));
    }
    if scope == Scope::Release && stage != Stage::Stable {
        return Err(DefaultError::new(messages::RELEASE_STAGE));
    }
    Ok(())
}

fn target_from_branch(branch: &str) -> Result<Option<ReleaseTarget>> {
    let (value, scope) = if let Some(value) = branch.strip_prefix("hotfix/") {
        (value.to_string(), Scope::Patch)
    } else if let Some(value) = branch.strip_prefix("release/") {
        if let Some(major) = value.strip_suffix(".x.x") {
            (format!("{major}.0.0"), Scope::Major)
        } else if let Some(minor) = value.strip_suffix(".x") {
            (format!("{minor}.0"), Scope::Minor)
        } else {
            return Err(DefaultError::new(messages::unsupported_target_branch(
                branch,
            )));
        }
    } else {
        return Ok(None);
    };
    if value.starts_with('v') {
        return Err(DefaultError::new(messages::unsupported_target_branch(
            branch,
        )));
    }
    let target =
        ReleaseTarget::parse(&value).context(messages::unsupported_target_branch(branch))?;
    if target.scope != scope {
        return Err(DefaultError::new(messages::unsupported_target_branch(
            branch,
        )));
    }
    Ok(Some(target))
}

/// Resolve target constraints separately from configurable scope/stage patterns.
/// Explicit options may supply a target, but cannot override a branch's target.
pub(crate) fn resolve_target(
    branch: &str,
    explicit: Option<&str>,
    scope: Scope,
) -> Result<Option<ReleaseTarget>> {
    let branch_target = target_from_branch(branch)?;
    let explicit_target = explicit.map(ReleaseTarget::parse).transpose()?;
    if let (Some(branch_target), Some(explicit_target)) = (&branch_target, &explicit_target) {
        if branch_target.version != explicit_target.version {
            return Err(DefaultError::new(messages::branch_target_conflict(
                &explicit_target.version.to_string(true),
                branch,
                &branch_target.version.to_string(true),
            )));
        }
    }
    let target = branch_target.or(explicit_target);
    if let Some(target) = &target {
        if scope != Scope::Release && scope != target.scope {
            return Err(DefaultError::new(messages::scope_target_conflict(
                scope.as_str(),
                &target.version.to_string(true),
                branch,
                target.scope.as_str(),
            )));
        }
    }
    Ok(target)
}
