use crate::branch_rules::Scope;
use crate::default_error::{DefaultError, Result, ResultExt};
use crate::error_messages as messages;
use crate::semantic_version::SemanticVersion;

/// An exact core version and the scope of its previous official release line.
pub(crate) struct ReleaseTarget {
    pub version: SemanticVersion,
    pub scope: Scope,
}

impl ReleaseTarget {
    fn parse(value: &str) -> Result<Self> {
        let version = SemanticVersion::from_string(value.to_string())
            .context(messages::invalid_target(value))?;
        if !version.prerelease_stage.is_empty()
            || version.to_string(value.starts_with('v')) != value
            || version == SemanticVersion::default()
        {
            return Err(DefaultError::new(messages::invalid_target(value))
                .with_source(DefaultError::new(messages::TARGET_FORMAT)));
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

    fn from_branch(branch: &str) -> Result<Option<Self>> {
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
        let target = Self::parse(&value).context(messages::unsupported_target_branch(branch))?;
        if target.scope != scope {
            return Err(DefaultError::new(messages::unsupported_target_branch(
                branch,
            )));
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
            // Targets are constructed with a bump scope by parse().
            Scope::Release => unreachable!("{}", messages::TARGET_BUMP_SCOPE),
        }
    }
}

/// Resolve target constraints separately from configurable scope/stage patterns.
/// Explicit options may supply a target, but cannot override a branch's target.
pub(crate) fn resolve(
    branch: &str,
    explicit: Option<&str>,
    scope: Scope,
) -> Result<Option<ReleaseTarget>> {
    let branch_target = ReleaseTarget::from_branch(branch)?;
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
