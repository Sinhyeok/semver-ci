use std::cmp::Ordering;

#[derive(Eq, PartialEq, Debug)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease_stage: String,
    pub prerelease_number: u64,
    pub commit_short_sha: String,
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
            .then_with(|| compare_prerelease_stage(&self.prerelease_stage, &other.prerelease_stage))
            .then_with(|| self.prerelease_number.cmp(&other.prerelease_number))
    }
}

fn compare_prerelease_stage(stage1: &String, stage2: &String) -> Ordering {
    match (stage1.is_empty(), stage2.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => stage1.cmp(stage2),
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
            commit_short_sha: self.commit_short_sha.clone(),
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

    pub fn increase_by_scope(&mut self, scope: String) -> SemanticVersion {
        let mut increased = self.clone();

        match scope.as_str() {
            "major" => increased.increase_major(),
            "minor" => increased.increase_minor(),
            "patch" => increased.increase_patch(),
            "prerelease" => increased.increase_prerelease_number(),
            _ => {
                panic!("Invalid scope: {}", scope)
            }
        }

        increased
    }

    pub fn release(&mut self) -> SemanticVersion {
        let mut release_version = self.clone();

        release_version.prerelease_stage = "".to_string();
        release_version.prerelease_number = 0;
        release_version.commit_short_sha = "".to_string();

        release_version
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
        let (prerelease_stage, prerelease_number, commit_short_sha) =
            if version_n_metadata.len() < 2 {
                ("".to_string(), 0, "".to_string())
            } else {
                metadata(version_n_metadata[1])?
            };

        Ok(SemanticVersion {
            major,
            minor,
            patch,
            prerelease_stage,
            prerelease_number,
            commit_short_sha,
        })
    }

    pub fn to_string(&self, prefix_v: bool) -> String {
        let version_string = match self.prerelease_stage.as_str() {
            "dev" => format!(
                "{}.{}.{}-{}.{}.{}",
                self.major,
                self.minor,
                self.patch,
                self.prerelease_stage,
                self.prerelease_number,
                self.commit_short_sha
            ),
            "rc" => format!(
                "{}.{}.{}-{}.{}",
                self.major, self.minor, self.patch, self.prerelease_stage, self.prerelease_number
            ),
            _ => format!("{}.{}.{}", self.major, self.minor, self.patch),
        };

        if prefix_v {
            format!("v{}", version_string)
        } else {
            version_string
        }
    }

    pub fn default() -> Self {
        SemanticVersion {
            major: 0,
            minor: 0,
            patch: 0,
            prerelease_stage: "".to_string(),
            prerelease_number: 0,
            commit_short_sha: "".to_string(),
        }
    }
}

fn version_part(part: &str, scope: &str) -> Result<u64, String> {
    part.parse::<u64>()
        .map_err(|e| format!("Invalid {} version: {}\n{}", scope, part, e))
}

fn metadata(metadata_string: &str) -> Result<(String, u64, String), String> {
    let metadata_parts: Vec<&str> = metadata_string.split('.').collect();
    if metadata_parts.len() < 2 {
        return Err(format!("Invalid metadata format: {}", metadata_string));
    }

    let prerelease_stage = metadata_parts[0].to_string();
    let prerelease_number = metadata_parts[1].parse::<u64>().map_err(|_| {
        format!(
            "Invalid prerelease number: {}, Metadata: {}",
            metadata_parts[1], metadata_string
        )
    })?;
    let short_commit_sha = metadata_parts.get(2).unwrap_or(&"").to_string();

    Ok((prerelease_stage, prerelease_number, short_commit_sha))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(version: &str) -> SemanticVersion {
        SemanticVersion::from_string(version.to_string()).unwrap()
    }

    #[test]
    fn supported_versions_round_trip_with_and_without_prefix() {
        for version in ["0.0.0", "12.34.56", "1.2.3-rc.10", "1.2.3-dev.7.abcd1234"] {
            let prefixed = format!("v{version}");
            for input in [version, prefixed.as_str()] {
                let parsed = parse(input);
                assert_eq!(parsed.to_string(false), version, "input: {input}");
                assert_eq!(parsed.to_string(true), prefixed, "input: {input}");
            }
        }
    }

    #[test]
    fn invalid_versions_report_the_invalid_component() {
        for (input, expected) in [
            ("", "Invalid version string format"),
            ("1.2", "Invalid version string format"),
            ("1.2.3.4", "Invalid version string format"),
            ("x.2.3", "Invalid major version"),
            ("1.x.3", "Invalid minor version"),
            ("1.2.x", "Invalid patch version"),
            ("18446744073709551616.2.3", "Invalid major version"),
            ("1.18446744073709551616.3", "Invalid minor version"),
            ("1.2.18446744073709551616", "Invalid patch version"),
            ("1.2.3-", "Invalid metadata format"),
            ("1.2.3-rc", "Invalid metadata format"),
            ("1.2.3-rc.x", "Invalid prerelease number"),
            ("1.2.3-rc.", "Invalid prerelease number"),
            ("1.2.3-rc.18446744073709551616", "Invalid prerelease number"),
        ] {
            let error = SemanticVersion::from_string(input.to_string()).unwrap_err();
            assert!(error.contains(expected), "input: {input}, error: {error}");
        }
    }

    #[test]
    fn version_components_accept_u64_max() {
        let version = parse("18446744073709551615.18446744073709551615.18446744073709551615-rc.18446744073709551615");
        assert_eq!(version.major, u64::MAX);
        assert_eq!(version.minor, u64::MAX);
        assert_eq!(version.patch, u64::MAX);
        assert_eq!(version.prerelease_number, u64::MAX);
    }

    #[test]
    fn ordering_uses_numeric_components_then_prerelease_precedence() {
        for (lower, higher) in [
            ("2.99.99", "10.0.0"),
            ("1.2.99", "1.10.0"),
            ("1.2.9", "1.2.10"),
            ("1.2.3-dev.99.abcdef12", "1.2.3-rc.1"),
            ("1.2.3-rc.2", "1.2.3-rc.10"),
            ("1.2.3-dev.2.ffffffff", "1.2.3-dev.10.00000000"),
            ("1.2.3-rc.99", "1.2.3"),
            ("1.2.3", "1.2.4-dev.1.abcdef12"),
        ] {
            let (lower, higher) = (parse(lower), parse(higher));
            assert_eq!(lower.cmp(&higher), Ordering::Less);
            assert_eq!(higher.cmp(&lower), Ordering::Greater);
            assert_eq!(lower.partial_cmp(&higher), Some(Ordering::Less));
        }
        let version = parse("1.2.3");
        assert_eq!(version.cmp(&version), Ordering::Equal);
    }

    #[test]
    fn scope_bumps_reset_lower_components_and_preserve_the_source() {
        for (scope, expected) in [("major", "2.0.0"), ("minor", "1.3.0"), ("patch", "1.2.4")] {
            let mut source = parse("1.2.3");
            assert_eq!(source.increase_by_scope(scope.to_string()), parse(expected));
            assert_eq!(source, parse("1.2.3"));
        }
    }

    #[test]
    fn prerelease_bump_preserves_stage_sha_and_source() {
        let mut source = parse("1.2.3-dev.9.abcdef12");
        assert_eq!(
            source.increase_by_scope("prerelease".to_string()),
            parse("1.2.3-dev.10.abcdef12")
        );
        assert_eq!(source, parse("1.2.3-dev.9.abcdef12"));
    }

    #[test]
    fn release_clears_all_prerelease_fields_and_preserves_the_source() {
        let mut source = parse("1.2.3-dev.9.abcdef12");
        let mut released = source.release();
        assert_eq!(released, parse("1.2.3"));
        assert_eq!(released.release(), released);
        assert_eq!(source, parse("1.2.3-dev.9.abcdef12"));
    }

    #[test]
    #[should_panic(expected = "Invalid scope: invalid")]
    fn unsupported_scope_is_rejected() {
        parse("1.2.3").increase_by_scope("invalid".to_string());
    }

    #[test]
    fn default_is_an_official_zero_version() {
        assert_eq!(SemanticVersion::default(), parse("0.0.0"));
    }

    #[test]
    fn parse_official_and_prerelease_versions() {
        let v = SemanticVersion::from_string("1.2.3".to_string()).unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease_stage, "");
        assert_eq!(v.prerelease_number, 0);
        assert_eq!(v.commit_short_sha, "");

        let rc = SemanticVersion::from_string("1.2.3-rc.4".to_string()).unwrap();
        assert_eq!(rc.prerelease_stage, "rc");
        assert_eq!(rc.prerelease_number, 4);
        assert_eq!(rc.to_string(true), "v1.2.3-rc.4");

        let dev = SemanticVersion::from_string("1.2.3-dev.7.abcd1234".to_string()).unwrap();
        assert_eq!(dev.prerelease_stage, "dev");
        assert_eq!(dev.prerelease_number, 7);
        assert_eq!(dev.commit_short_sha, "abcd1234");
        assert_eq!(dev.to_string(false), "1.2.3-dev.7.abcd1234");
    }

    #[test]
    fn increase_and_release_behaviors() {
        let mut v = SemanticVersion::from_string("1.2.3-rc.1".to_string()).unwrap();

        let minor = v.increase_by_scope("minor".to_string());
        assert_eq!(minor.to_string(false), "1.3.0-rc.1");

        let patch = v.increase_by_scope("patch".to_string());
        assert_eq!(patch.to_string(false), "1.2.4-rc.1");

        let pre = v.increase_by_scope("prerelease".to_string());
        assert_eq!(pre.to_string(false), "1.2.3-rc.2");

        let rel = v.release();
        assert_eq!(rel.to_string(true), "v1.2.3");
    }
}
