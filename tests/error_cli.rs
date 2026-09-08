mod common;

use assert_cmd::{assert::Assert, prelude::*};
use common::TestRepo;
use predicates::prelude::*;

fn assert_error(result: Assert, expected: &str) {
    result
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("Error: "))
        .stderr(predicate::str::contains(expected))
        .stderr(predicate::str::contains("panicked at").not())
        .stderr(predicate::str::contains("stack backtrace").not());
}

#[test]
fn missing_environment_variables_are_returned_by_local_and_ci_pipelines() {
    let repo = TestRepo::new("develop");
    assert_error(repo.command().env_remove("GIT_TOKEN").arg("version").assert(),
        "Failed to read environment variable 'GIT_TOKEN'\n    Caused by: environment variable not found");
    for (pipeline, variable) in [
        ("GITHUB_ACTIONS", "GITHUB_REF_NAME"),
        ("GITLAB_CI", "CI_COMMIT_REF_NAME"),
    ] {
        assert_error(
            repo.command().env(pipeline, "true").arg("scope").assert(),
            &format!("Failed to read environment variable '{variable}'"),
        );
    }
}

#[test]
fn malformed_boolean_is_an_error_instead_of_a_default() {
    assert_error(
        TestRepo::new("develop")
            .command()
            .env("FORCE_FETCH_TAGS", "yes")
            .arg("version")
            .assert(),
        "Invalid boolean for FORCE_FETCH_TAGS: 'yes'; expected true or false",
    );
}

#[cfg(unix)]
#[test]
fn non_unicode_optional_environment_is_an_error_instead_of_a_default() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    assert_error(
        TestRepo::new("develop")
            .command()
            .env("FORCE_FETCH_TAGS", OsString::from_vec(vec![0xff]))
            .arg("version")
            .assert(),
        "Failed to read environment variable 'FORCE_FETCH_TAGS'",
    );
}

#[test]
fn invalid_authentication_headers_do_not_panic_or_print_tokens() {
    let repo = TestRepo::new("main");
    for (pipeline, token, header) in [
        ("GITHUB_ACTIONS", "GITHUB_TOKEN", "Authorization"),
        ("GITLAB_CI", "CI_JOB_TOKEN", "JOB-TOKEN"),
    ] {
        let result = repo
            .command()
            .env(pipeline, "true")
            .env("GITHUB_API_URL", "http://localhost")
            .env("GITHUB_REPOSITORY", "test/repo")
            .env("CI_API_V4_URL", "http://localhost")
            .env("CI_PROJECT_ID", "123")
            .env(token, "private-token\ninvalid")
            .args(["release", "v1.2.3"])
            .assert()
            .stderr(predicate::str::contains("private-token").not());
        assert_error(result, &format!("Invalid HTTP header '{header}'"));
    }
}

#[test]
fn short_or_non_ascii_github_sha_is_rejected() {
    let repo = TestRepo::new("develop");
    let config_root = repo.repo.workdir().unwrap().join("test-config");
    std::fs::create_dir_all(config_root.join("git")).unwrap();
    std::fs::write(config_root.join("git/config"), "").unwrap();
    for sha in ["abc", "1234567é", "abcdefgh"] {
        assert_error(
            repo.command()
                .env("XDG_CONFIG_HOME", &config_root)
                .env("GITHUB_ACTIONS", "true")
                .env("GITHUB_REF_NAME", "develop")
                .env("GITHUB_SHA", sha)
                .arg("version")
                .assert(),
            "Invalid commit SHA in GITHUB_SHA",
        );
    }
}

#[test]
fn malformed_dotenv_is_reported() {
    let repo = TestRepo::new("develop");
    std::fs::write(repo.repo.workdir().unwrap().join(".env"), "=invalid\n").unwrap();
    assert_error(
        repo.command().arg("scope").assert(),
        "Failed to load .env configuration",
    );
}

#[test]
fn git_errors_keep_their_context_and_cause() {
    let repo = TestRepo::new("develop");
    assert_error(repo.command().env("FORCE_FETCH_TAGS", "true").arg("version").assert(),
        "Error: Failed to retrieve tags\n    Caused by: Failed to fetch Git references from origin: refs/tags/*:refs/tags/*\n    Caused by:");
    assert_error(
        repo.command().args(["tag", "v1.2.3"]).assert(),
        "Failed to push tag 'v1.2.3'\n    Caused by:",
    );
}

#[test]
fn unsupported_release_pipeline_is_an_application_error() {
    assert_error(
        TestRepo::new("main")
            .command()
            .args(["release", "v1.2.3"])
            .assert(),
        "Not supported pipeline: Git Repo",
    );
}

#[test]
fn invalid_cli_arguments_keep_the_usage_exit_code() {
    TestRepo::new("develop")
        .command()
        .args(["version", "--scope", "invalid"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn github_initialization_errors_are_returned_before_running_the_command() {
    assert_error(
        TestRepo::new("develop")
            .command()
            .env("XDG_CONFIG_HOME", "/dev/null")
            .env("GITHUB_ACTIONS", "true")
            .arg("version")
            .assert(),
        "Git configuration",
    );
}

#[test]
fn failed_http_requests_are_reported() {
    assert_error(
        TestRepo::new("main")
            .command()
            .env("ENVIRONMENT", "production")
            .env("GITHUB_ACTIONS", "true")
            .env("GITHUB_API_URL", ":invalid-url")
            .env("GITHUB_REPOSITORY", "test/repo")
            .env("GITHUB_TOKEN", "test-token")
            .env("GITHUB_SHA", "12345678")
            .args(["release", "v1.2.3"])
            .assert(),
        "Failed to send HTTP request\n    Caused by:",
    );
}

#[test]
fn gitlab_token_override_does_not_require_the_fallback_token() {
    let repo = TestRepo::new("develop");
    let origin = tempfile::TempDir::new().unwrap();
    git2::Repository::init_bare(origin.path()).unwrap();
    repo.repo
        .remote("origin", origin.path().to_str().unwrap())
        .unwrap();
    repo.command()
        .env("GITLAB_CI", "true")
        .env("CI_PROJECT_URL", "https://example.com/test/repo")
        .env("CI_COMMIT_REF_NAME", "develop")
        .env("CI_COMMIT_SHORT_SHA", "12345678")
        .env("CI_COMMIT_SHA", "HEAD")
        .env("GITLAB_USER_EMAIL", "test@example.com")
        .env("SEMVER_CI_TOKEN", "test-token")
        .env_remove("CI_JOB_TOKEN")
        .arg("version")
        .assert()
        .success()
        .stdout("UPCOMING_VERSION=v0.1.0-dev.1.12345678\nLAST_VERSION=v0.0.0\n");
}

#[test]
fn version_overflow_reaches_the_cli_without_partial_output() {
    let repo = TestRepo::new("develop");
    let object = repo
        .repo
        .head()
        .unwrap()
        .peel(git2::ObjectType::Commit)
        .unwrap();
    repo.repo
        .tag_lightweight("v1.18446744073709551615.0", &object, false)
        .unwrap();
    assert_error(
        repo.command().arg("version").assert(),
        "Cannot increase minor version: numeric component exceeds u64::MAX",
    );
}
