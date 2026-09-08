//! Application-owned error and recovery messages. Dependency diagnostics remain sources.
use std::fmt::Display;

pub(crate) const LOAD_ENV: &str = "Failed to load .env configuration";
pub(crate) const INIT_LOGGER: &str = "Failed to initialize logging";
pub(crate) const RETRIEVE_TAGS: &str = "Failed to retrieve tags";
pub(crate) const RETRIEVE_BRANCH: &str = "Failed to retrieve branch name";
pub(crate) const RETRIEVE_SHA: &str = "Failed to retrieve short commit SHA";
pub(crate) const DETACHED_HEAD: &str = "HEAD is in detached state, not pointing to branch";
pub(crate) const COMPLETE_HISTORY: &str = "Version calculation requires complete history. Run git fetch --unshallow --tags, or configure a full CI checkout.";
pub(crate) const READ_GIT_CONFIG: &str = "Failed to read Git configuration";
pub(crate) const HTTP_CLIENT: &str = "Failed to create HTTP client";
pub(crate) const HTTP_SEND: &str = "Failed to send HTTP request";
pub(crate) const HTTP_JSON: &str = "Failed to parse HTTP response as a JSON object";
pub(crate) const CANDIDATE_STAGE: &str =
    "--candidate requires official version calculation; use --stage stable or --scope release.";
pub(crate) const CANDIDATE_SCOPE: &str = "--candidate cannot be combined with a major, minor, or patch scope; omit scope or use release.";
pub(crate) const RELEASE_STAGE: &str =
    "scope release requires stage stable; select an increase scope for dev or rc.";
pub(crate) const CANDIDATE_NOT_FOUND: &str = "tag not found; fetch the exact tag before retrying";
pub(crate) const CANDIDATE_FORMAT: &str = "expected a valid dev or rc prerelease tag";
pub(crate) const CANDIDATE_COMMIT: &str = "tag must point to a commit";
pub(crate) const TARGET_FORMAT: &str =
    "expected a nonzero official version X.Y.Z, optionally prefixed with v.";
pub(crate) const TARGET_BUMP_SCOPE: &str = "a target always has a bump scope";

pub(crate) fn error_report(error: impl Display) -> String {
    format!("Error: {error}")
}

pub(crate) fn caused_by(error: impl Display) -> String {
    format!("\n    Caused by: {error}")
}

pub(crate) fn environment(name: &str) -> String {
    format!("Failed to read environment variable '{name}'")
}

pub(crate) fn invalid_boolean(name: &str, value: &str) -> String {
    format!("Invalid boolean for {name}: '{value}'; expected true or false")
}

pub(crate) fn invalid_commit_sha(name: &str) -> String {
    format!("Invalid commit SHA in {name}: expected at least 8 ASCII hexadecimal characters")
}

pub(crate) fn invalid_header(name: &str) -> String {
    format!("Invalid HTTP header '{name}'")
}

pub(crate) fn unsupported_pipeline(name: &str) -> String {
    format!("Not supported pipeline: {name}")
}

pub(crate) fn open_repository(path: &str) -> String {
    format!("Failed to open Git repository at '{path}'")
}

pub(crate) fn clone_repository(path: &str) -> String {
    format!("Failed to clone Git repository into '{path}'")
}

pub(crate) fn read_git_config(name: &str) -> String {
    format!("Failed to read Git configuration '{name}'")
}

pub(crate) fn set_git_config(name: &str) -> String {
    format!("Failed to set Git configuration '{name}'")
}

pub(crate) fn checkout(reference: &str) -> String {
    format!("Failed to check out Git reference '{reference}'")
}

pub(crate) fn fetch_refs(refs: &[&str]) -> String {
    format!(
        "Failed to fetch Git references from origin: {}",
        refs.join(", ")
    )
}

pub(crate) fn tag_commit(tag: &str) -> String {
    format!("Failed to resolve commit for tag '{tag}'")
}

pub(crate) fn target_commit(target: &str) -> String {
    format!("Failed to resolve target commit '{target}'")
}

pub(crate) fn check_ancestry(tag: &str) -> String {
    format!("Failed to check ancestry for tag '{tag}'")
}

pub(crate) fn create_tag(tag: &str) -> String {
    format!("Failed to create tag '{tag}'")
}

pub(crate) fn push_tag(tag: &str) -> String {
    format!("Failed to push tag '{tag}'")
}

pub(crate) fn unsupported_credentials(credential: impl std::fmt::Debug) -> String {
    format!("Unsupported Git credential type: {credential:?}")
}

pub(crate) fn http_status(
    status: impl Display,
    headers: impl std::fmt::Debug,
    body: &str,
) -> String {
    format!("HTTP request failed. Status: {status}\nHeaders:\n{headers:#?}\nBody:\n{body}")
}

pub(crate) fn http_body(status: impl Display) -> String {
    format!("Failed to read HTTP response body (Status: {status})")
}

pub(crate) fn unknown_branch(branch: &str) -> String {
    format!("Unknown branch name: {branch}. Configure MAJOR/MINOR/PATCH/RELEASE or pass --scope.")
}

pub(crate) fn unknown_stage(branch: &str) -> String {
    format!("Unknown stage for branch: {branch}. Configure DEV/RC/STABLE or pass --stage.")
}

pub(crate) fn invalid_regex(pattern: &str) -> String {
    format!("Invalid regular expression: '{pattern}'")
}

pub(crate) fn invalid_scope(scope: &str) -> String {
    format!("Invalid scope: {scope}")
}

pub(crate) fn version_overflow(scope: &str) -> String {
    format!("Cannot increase {scope} version: numeric component exceeds u64::MAX")
}

pub(crate) fn invalid_version(version: &str) -> String {
    format!("Invalid version string format: {version}")
}

pub(crate) fn invalid_version_part(scope: &str, part: &str) -> String {
    format!("Invalid {scope} version: {part}")
}

pub(crate) fn invalid_metadata(metadata: &str) -> String {
    format!("Invalid metadata format: {metadata}")
}

pub(crate) fn invalid_prerelease(number: &str, metadata: &str) -> String {
    format!("Invalid prerelease number: {number}, Metadata: {metadata}")
}

pub(crate) fn invalid_target(value: &str) -> String {
    format!("Invalid target '{value}'")
}

pub(crate) fn unsupported_target_branch(branch: &str) -> String {
    format!("Unsupported release target branch '{branch}': use hotfix/M.m.p (p > 0), release/M.m.x (m > 0), or release/M.x.x (M > 0), with canonical numeric components.")
}

pub(crate) fn branch_target_conflict(explicit: &str, branch: &str, target: &str) -> String {
    format!("Target {explicit} conflicts with branch '{branch}' target {target}.")
}

pub(crate) fn scope_target_conflict(
    scope: &str,
    target: &str,
    branch: &str,
    expected: &str,
) -> String {
    format!("Scope {scope} conflicts with target {target} on branch '{branch}'; use scope {expected} or stable candidate promotion.")
}

pub(crate) fn target_not_newer(target: &str, last: &str) -> String {
    format!("Target {target} must be newer than the release line's last official version ({last}).")
}

pub(crate) fn target_released(target: &str, tag: &str) -> String {
    format!("Target {target} is already released as tag '{tag}'. Select a new target.")
}

pub(crate) fn candidate_target_conflict(candidate: &str, target: &str) -> String {
    format!("Candidate version {candidate} conflicts with target {target}.")
}

pub(crate) fn no_target_base(target: &str) -> String {
    format!("No reachable official base in the release line for target {target}. Fetch complete tags/history and check the branch base.")
}

pub(crate) fn no_target_prerelease(target: &str, scope: &str) -> String {
    format!("No reachable prerelease for target {target}. Select an exact --candidate, or use --scope {scope} --stage stable to calculate the target directly.")
}

pub(crate) fn minor_bump_fallback(last: &str) -> String {
    format!(
        "No newer reachable pre-release after last official tag ({last}). Fallback to minor bump."
    )
}

pub(crate) fn ambiguous_release(tags: &str) -> String {
    format!("Ambiguous official release: reachable prerelease tags target multiple versions: {tags}. Select one with --candidate <tag>.")
}

pub(crate) fn invalid_candidate(candidate: &str) -> String {
    format!("Invalid candidate '{candidate}'")
}

pub(crate) fn candidate_not_newer(last: &str) -> String {
    format!("version must be newer than the target's last official version ({last})")
}

pub(crate) fn candidate_released(tag: &str) -> String {
    format!("official version already exists as tag '{tag}'")
}
