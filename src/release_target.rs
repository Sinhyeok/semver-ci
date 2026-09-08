use crate::branch_rules::Scope;
use crate::default_error::DefaultError;
use crate::semantic_version::SemanticVersion;
use std::error::Error;

/// An exact core version and the scope of its previous official release line.
pub(crate) struct ReleaseTarget {
    pub version: SemanticVersion,
    pub scope: Scope,
}

impl ReleaseTarget {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        let version = SemanticVersion::from_string(value.to_string())
            .map_err(|reason| target_error(format!("Invalid target '{value}': {reason}")))?;
        if !version.prerelease_stage.is_empty()
            || version.to_string(value.starts_with('v')) != value
            || version == SemanticVersion::default()
        {
            return Err(target_error(format!(
                "Invalid target '{value}': expected a nonzero official version X.Y.Z, optionally prefixed with v."
            )));
        }
        let scope = if version.patch > 0 {
            Scope::Patch
        } else if version.minor > 0 {
            Scope::Minor
        } else {
            Scope::Major
        };
        Ok(Self { version, scope })
    }

    fn from_branch(branch: &str) -> Result<Option<Self>, Box<dyn Error>> {
        let (value, scope) = if let Some(value) = branch.strip_prefix("hotfix/") {
            (value.to_string(), Scope::Patch)
        } else if let Some(value) = branch.strip_prefix("release/") {
            if let Some(major) = value.strip_suffix(".x.x") {
                (format!("{major}.0.0"), Scope::Major)
            } else if let Some(minor) = value.strip_suffix(".x") {
                (format!("{minor}.0"), Scope::Minor)
            } else {
                return Err(unsupported_branch(branch));
            }
        } else {
            return Ok(None);
        };
        if value.starts_with('v') {
            return Err(unsupported_branch(branch));
        }
        let target = Self::parse(&value).map_err(|_| unsupported_branch(branch))?;
        if target.scope != scope {
            return Err(unsupported_branch(branch));
        }
        Ok(Some(target))
    }

    /// Patch releases use the same major/minor; minor releases use the same
    /// major; major releases may start from any earlier major.
    pub(crate) fn includes_base(&self, version: &SemanticVersion) -> bool {
        match self.scope {
            Scope::Patch => {
                version.major == self.version.major && version.minor == self.version.minor
            }
            Scope::Minor => version.major == self.version.major,
            Scope::Major => true,
            Scope::Release => unreachable!("a target always has a bump scope"),
        }
    }
}

/// Resolve target constraints separately from configurable scope/stage patterns.
/// Explicit options may supply a target, but cannot override a branch's target.
pub(crate) fn resolve(
    branch: &str,
    explicit: Option<&str>,
    scope: Scope,
) -> Result<Option<ReleaseTarget>, Box<dyn Error>> {
    let branch_target = ReleaseTarget::from_branch(branch)?;
    let explicit_target = explicit.map(ReleaseTarget::parse).transpose()?;
    if let (Some(branch_target), Some(explicit_target)) = (&branch_target, &explicit_target) {
        if branch_target.version != explicit_target.version {
            return Err(target_error(format!(
                "Target {} conflicts with branch '{branch}' target {}.",
                explicit_target.version.to_string(true),
                branch_target.version.to_string(true)
            )));
        }
    }
    let target = branch_target.or(explicit_target);
    if let Some(target) = &target {
        if scope != Scope::Release && scope != target.scope {
            return Err(target_error(format!(
                "Scope {} conflicts with target {} on branch '{branch}'; use scope {} or stable candidate promotion.",
                scope.as_str(), target.version.to_string(true), target.scope.as_str()
            )));
        }
    }
    Ok(target)
}

fn unsupported_branch(branch: &str) -> Box<dyn Error> {
    target_error(format!(
        "Unsupported release target branch '{branch}': use hotfix/M.m.p (p > 0), release/M.m.x (m > 0), or release/M.x.x (M > 0), with canonical numeric components."
    ))
}

pub(crate) fn target_error(message: String) -> Box<dyn Error> {
    DefaultError {
        message,
        source: None,
    }
    .into()
}
