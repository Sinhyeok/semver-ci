use std::cmp::Ordering;
use std::ops::Not;

#[derive(Eq, PartialEq, Debug)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease_stage: String,
    pub prerelease_number: u64,
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.prerelease_number.cmp(&other.prerelease_number))
    }
}

impl Clone for SemanticVersion {
    fn clone(&self) -> Self {
        SemanticVersion {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            prerelease_stage: self.prerelease_stage.clone(),
            prerelease_number: self.prerelease_number,
        }
    }
}

impl SemanticVersion {
    fn increase_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    fn increase_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    fn increase_patch(&mut self) {
        self.patch += 1;
    }

    fn increase_prerelease_number(&mut self) {
        self.prerelease_number += 1;
    }

    pub fn increase_by_scope(&mut self, scope: String) -> &mut SemanticVersion {
        match scope.as_str() {
            "major" => self.increase_major(),
            "minor" => self.increase_minor(),
            "patch" => self.increase_patch(),
            "prerelease" => self.increase_prerelease_number(),
            _ => {
                panic!("Invalid scope: {}", scope)
            }
        }

        self
    }

    pub fn from_string(version_string: String) -> Result<Self, String> {
        let prefix_stripped = match version_string.strip_prefix('v') {
            Some(stripped) => stripped.to_string(),
            None => version_string.clone(),
        };

        let version_n_metadata: Vec<&str> = prefix_stripped.split('-').collect();

        // version
        let version_parts: Vec<&str> = version_n_metadata[0].split('.').collect();
        if version_parts.len() != 3 {
            return Err(format!("Invalid version string format: {}", version_string));
        }

        let major = version_part(version_parts[0], "major")?;
        let minor = version_part(version_parts[1], "minor")?;
        let patch = version_part(version_parts[2], "patch")?;

        // metadata
        let (prerelease_stage, prerelease_number) = if version_n_metadata.len() < 2 {
            ("".to_string(), 0)
        } else {
            metadata(version_n_metadata[1])?
        };

        Ok(SemanticVersion {
            major,
            minor,
            patch,
            prerelease_stage,
            prerelease_number,
        })
    }

    pub fn to_string(&self, prefix_v: bool) -> String {
        let version_string = if self.is_prerelease() {
            format!(
                "{}.{}.{}-{}.{}",
                self.major, self.minor, self.patch, self.prerelease_stage, self.prerelease_number
            )
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        };

        if prefix_v {
            format!("v{}", version_string)
        } else {
            version_string
        }
    }

    fn is_prerelease(&self) -> bool {
        self.prerelease_stage.is_empty().not()
    }
}

fn version_part(part: &str, scope: &str) -> Result<u64, String> {
    part.parse::<u64>()
        .map_err(|e| format!("Invalid {} version: {}\n{}", scope, part, e))
}

fn metadata(metadata_string: &str) -> Result<(String, u64), String> {
    let metadata_parts: Vec<&str> = metadata_string.split('.').collect();
    if metadata_parts.len() < 2 {
        return Err(format!("Invalid metadata format: {}", metadata_string));
    }

    let prerelease_stage = metadata_parts[0].to_string();
    let prerelease_number = metadata_parts[1]
        .parse::<u64>()
        .map_err(|e| format!("Invalid prerelease number: {}\n{}", metadata_parts[1], e))?;

    Ok((prerelease_stage, prerelease_number))
}
