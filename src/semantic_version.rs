use std::cmp::Ordering;

#[derive(Eq, PartialEq, Debug)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
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
    }
}

impl SemanticVersion {
    pub fn increase_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    pub fn increase_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    pub fn increase_patch(&mut self) {
        self.patch += 1;
    }

    pub fn increase_by_scope(&mut self, scope: String) -> &mut SemanticVersion {
        match scope.as_str() {
            "major" => self.increase_major(),
            "minor" => self.increase_minor(),
            "patch" => self.increase_patch(),
            _ => {
                panic!("Invalid scope: {}", scope)
            }
        }

        self
    }

    pub fn from_string(version_string: String) -> Result<Self, String> {
        let prefix_stripped = match version_string.strip_prefix('v') {
            Some(stripped) => stripped.to_string(),
            None => version_string,
        };

        let parts: Vec<&str> = prefix_stripped.split('.').collect();

        if parts.len() != 3 {
            return Err("Invalid version string format".to_string());
        }

        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| "Invalid major version")?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| "Invalid minor version")?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| "Invalid patch version")?;

        Ok(SemanticVersion {
            major,
            minor,
            patch,
        })
    }

    pub fn to_string(&self, prefix_v: bool) -> String {
        let version_string = format!("{}.{}.{}", self.major, self.minor, self.patch);

        if prefix_v {
            format!("v{}", version_string)
        } else {
            version_string
        }
    }
}
