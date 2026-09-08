use crate::config;
use crate::default_error::{DefaultError, Result, ResultExt};
use crate::error_messages as messages;
use clap::ValueEnum;
use regex::Regex;

// Preserve the legacy scope regexes and their first-match precedence.
pub(crate) const MAJOR_PATTERN: &str = r"^release/[0-9]+.x.x$";
pub(crate) const MINOR_PATTERN: &str = r"^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$";
pub(crate) const PATCH_PATTERN: &str = r"^hotfix/[0-9]+.[0-9]+.[0-9]+$";
pub(crate) const STABLE_PATTERN: &str = r"^(main|master)$";
const DEV_PATTERN: &str = r"^(develop|feature/.*)$";
const RC_PATTERN: &str = r"^(release|hotfix)/.*$";

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum Scope {
    Major,
    Minor,
    Patch,
    Release,
}

impl Scope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum Stage {
    Dev,
    Rc,
    Stable,
}

impl Stage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Rc => "rc",
            Self::Stable => "stable",
        }
    }
}

pub(crate) struct ScopePatterns {
    pub major: String,
    pub minor: String,
    pub patch: String,
    pub release: String,
}

impl ScopePatterns {
    fn from_env() -> Result<Self> {
        Ok(Self {
            major: config::env_var_or("MAJOR", MAJOR_PATTERN)?,
            minor: config::env_var_or("MINOR", MINOR_PATTERN)?,
            patch: config::env_var_or("PATCH", PATCH_PATTERN)?,
            release: config::env_var_or("RELEASE", STABLE_PATTERN)?,
        })
    }

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

fn resolve_stage(branch: &str) -> Result<Stage> {
    let dev = config::env_var_or("DEV", DEV_PATTERN)?;
    let rc = config::env_var_or("RC", RC_PATTERN)?;
    let stable = config::env_var_or("STABLE", STABLE_PATTERN)?;
    matching_rule(
        branch,
        &[
            (Stage::Dev, &dev),
            (Stage::Rc, &rc),
            (Stage::Stable, &stable),
        ],
    )?
    .ok_or_else(|| DefaultError::new(messages::unknown_stage(branch)))
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

/// CLI/environment values are already selected by clap. Infer only missing values.
pub(crate) fn resolve_policy(
    branch: &str,
    scope: Option<Scope>,
    stage: Option<Stage>,
    has_candidate: bool,
) -> Result<(Scope, Stage)> {
    let stage = match stage {
        Some(stage) => stage,
        // Preserve the existing explicit release scope on any named branch.
        None if scope == Some(Scope::Release) => Stage::Stable,
        None => resolve_stage(branch)?,
    };
    if has_candidate && stage != Stage::Stable {
        return Err(DefaultError::new(messages::CANDIDATE_STAGE));
    }

    let scope = match scope {
        Some(scope) => scope,
        None if has_candidate => Scope::Release,
        None => ScopePatterns::from_env()?.resolve(branch)?,
    };
    if has_candidate && scope != Scope::Release {
        return Err(DefaultError::new(messages::CANDIDATE_SCOPE));
    }
    if scope == Scope::Release && stage != Stage::Stable {
        return Err(DefaultError::new(messages::RELEASE_STAGE));
    }
    Ok((scope, stage))
}
