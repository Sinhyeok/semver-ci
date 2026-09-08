use crate::semantic_version::SemanticVersion;

pub(crate) struct Release {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) tag_name: String,
    pub(crate) tag_message: String,
    pub(crate) generate_release_notes: bool,
    pub(crate) previous_tag: String,
}

impl Release {
    pub(crate) fn is_prerelease(&self) -> bool {
        let Ok(version) = SemanticVersion::from_string(self.tag_name.clone()) else {
            return false;
        };
        let supported_stage = match version.prerelease_stage.as_str() {
            "rc" => true,
            "dev" => !version.commit_short_sha.is_empty(),
            _ => false,
        };
        // The parser tolerates extra fields; require the complete supported tag format.
        supported_stage && version.to_string(self.tag_name.starts_with('v')) == self.tag_name
    }
}
