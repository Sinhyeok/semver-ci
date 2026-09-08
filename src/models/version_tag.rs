use super::SemanticVersion;
use crate::errors::{messages, Result, ResultExt};
use regex::Regex;

const SEMANTIC_VERSION_TAG_OFFICIAL_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";
const SEMANTIC_VERSION_TAG_PRERELEASE_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+-.+)$";

#[derive(PartialEq, Eq)]
pub(crate) enum TagKind {
    Official,
    Prerelease,
    Other,
}

/// An original Git tag name, its parsed version, and its name-based classification.
pub(crate) struct VersionTag {
    pub(crate) name: String,
    pub(crate) version: SemanticVersion,
    pub(crate) kind: TagKind,
}

impl VersionTag {
    /// Skip unparseable tags while preserving original names and input order.
    pub(crate) fn parse_all(tag_names: &[String]) -> Result<Vec<Self>> {
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
                Some(Self {
                    name: name.clone(),
                    version,
                    kind,
                })
            })
            .collect())
    }
}
