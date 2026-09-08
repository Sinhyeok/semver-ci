use super::{Scope, SemanticVersion};
use crate::errors::{messages, DefaultError, Result, ResultExt};

/// An exact core version and the scope of its previous official release line.
pub(crate) struct ReleaseTarget {
    pub version: SemanticVersion,
    pub scope: Scope,
}

impl ReleaseTarget {
    pub(crate) fn parse(value: &str) -> Result<Self> {
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
