mod common;

use assert_cmd::prelude::*;
use common::TestRepo;
use predicates::prelude::*;
use std::process::Command;

fn repo_with_tags(branch: &str, tags: &[&str]) -> TestRepo {
    let repo = TestRepo::new(branch);
    let head = repo.repo.head().unwrap().peel_to_commit().unwrap();
    for tag in tags {
        repo.repo
            .tag_lightweight(tag, head.as_object(), false)
            .unwrap();
    }
    drop(head);
    repo
}

fn assert_version(repo: &TestRepo, command: &mut Command, upcoming: &str, last: &str) {
    let sha = repo
        .repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let upcoming = upcoming.replace("{sha}", &sha[..8]);
    command.assert().success().stdout(format!(
        "UPCOMING_VERSION={upcoming}\nLAST_VERSION={last}\n"
    ));
}

#[test]
fn branches_with_no_tags_start_from_zero() {
    for (branch, upcoming) in [
        ("develop", "v0.1.0-dev.1.{sha}"),
        ("feature/topic", "v0.1.0-dev.1.{sha}"),
        ("release/0.1.x", "v0.1.0-rc.1"),
        ("hotfix/0.0.1", "v0.1.0-rc.1"),
        ("main", "v0.1.0"),
        ("master", "v0.1.0"),
    ] {
        let repo = TestRepo::new(branch);
        assert_version(&repo, repo.command().arg("version"), upcoming, "v0.0.0");
    }
}

#[test]
fn feature_with_official_tag_only() {
    let repo = repo_with_tags("feature/xyz", &["v1.2.3"]);
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.3.0-dev.1.{sha}",
        "v1.2.3",
    );
}

#[test]
fn feature_with_prerelease_tag_advances_prerelease_and_replaces_sha() {
    let repo = repo_with_tags("feature/abc", &["v1.2.3", "v1.3.0-dev.1.abcd1234"]);
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.3.0-dev.2.{sha}",
        "v1.3.0-dev.1.abcd1234",
    );
}

#[test]
fn release_branch_with_prerelease_advances_release_candidate() {
    let repo = repo_with_tags("release/1.3.x", &["v1.2.3", "v1.3.0-rc.1"]);
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.3.0-rc.2",
        "v1.3.0-rc.1",
    );
}

#[test]
fn main_promotes_the_latest_prerelease_to_official() {
    let repo = repo_with_tags("main", &["v1.2.3", "v1.3.0-rc.2"]);
    assert_version(&repo, repo.command().arg("version"), "v1.3.0", "v1.2.3");
}

#[test]
fn main_falls_back_to_minor_when_no_newer_prerelease_exists() {
    for prereleases in [vec![], vec!["v1.2.3-rc.2"], vec!["v1.1.0-rc.9"]] {
        let mut tags = vec!["v1.2.3"];
        tags.extend(prereleases);
        let repo = repo_with_tags("main", &tags);
        assert_version(&repo, repo.command().arg("version"), "v1.3.0", "v1.2.3");
    }
}

#[test]
fn explicit_scopes_reset_the_appropriate_version_components() {
    for (branch, scope, upcoming) in [
        ("release/2.x.x", "major", "v2.0.0-rc.1"),
        ("develop", "minor", "v1.3.0-dev.1.{sha}"),
        ("hotfix/1.2.4", "patch", "v1.2.4-rc.1"),
    ] {
        let repo = repo_with_tags(branch, &["v1.2.3"]);
        assert_version(
            &repo,
            repo.command().args(["version", "--scope", scope]),
            upcoming,
            "v1.2.3",
        );
    }
}

#[test]
fn scope_environment_is_used_and_command_line_takes_precedence() {
    let repo = repo_with_tags("develop", &["v1.2.3"]);
    assert_version(
        &repo,
        repo.command().env("SCOPE", "patch").arg("version"),
        "v1.2.4-dev.1.{sha}",
        "v1.2.3",
    );
    assert_version(
        &repo,
        repo.command()
            .env("SCOPE", "patch")
            .args(["version", "--scope", "major"]),
        "v2.0.0-dev.1.{sha}",
        "v1.2.3",
    );
}

#[test]
fn release_scope_promotes_a_prerelease_even_on_develop() {
    let repo = repo_with_tags("develop", &["v1.2.3", "v2.0.0-rc.1"]);
    assert_version(
        &repo,
        repo.command().args(["version", "--scope", "release"]),
        "v2.0.0",
        "v1.2.3",
    );
}

#[test]
fn official_tags_are_selected_numerically_and_normalized_to_v_prefix() {
    let repo = repo_with_tags(
        "develop",
        &[
            "v1.9.99",
            "1.10.2",
            "v1.10.1",
            "not-a-version",
            "v99.0.0-backup",
        ],
    );
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.11.0-dev.1.{sha}",
        "v1.10.2",
    );
}

#[test]
fn prerelease_selection_uses_numeric_order_and_only_the_target_version_and_stage() {
    let repo = repo_with_tags(
        "develop",
        &[
            "v1.2.3",
            "v1.3.0-dev.2.ffffffff",
            "1.3.0-dev.10.abcd1234",
            "v1.3.0-rc.99",
            "v1.2.4-dev.99.12345678",
            "v2.0.0-dev.99.12345678",
        ],
    );
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.3.0-dev.11.{sha}",
        "v1.3.0-dev.10.abcd1234",
    );
}

#[test]
fn release_candidate_does_not_reuse_the_dev_counter() {
    let repo = repo_with_tags("release/1.3.x", &["v1.2.3", "v1.3.0-dev.99.abcd1234"]);
    assert_version(
        &repo,
        repo.command().arg("version"),
        "v1.3.0-rc.1",
        "v1.2.3",
    );
}

#[test]
fn malformed_semantic_tags_are_skipped_when_selecting_a_release() {
    let repo = repo_with_tags(
        "main",
        &[
            "v1.2.3",
            "v1.3.0-rc.10",
            "v1.3.0-rc.2",
            "v9.0.0-rc.invalid",
            "v10.0.0-rc",
            "v18446744073709551616.0.0",
            "unrelated",
        ],
    );
    assert_version(&repo, repo.command().arg("version"), "v1.3.0", "v1.2.3");
}

#[test]
fn invalid_scope_reports_an_error_without_version_output() {
    TestRepo::new("develop")
        .command()
        .args(["version", "--scope", "invalid"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Invalid scope: invalid"));
}

#[test]
fn forced_fetch_uses_tags_from_a_local_origin() {
    let origin = repo_with_tags("main", &["v1.9.0"]);
    let repo = repo_with_tags("develop", &["v1.2.3"]);
    repo.repo
        .remote("origin", origin.repo.path().to_str().unwrap())
        .unwrap();
    assert_version(
        &repo,
        repo.command()
            .env("FORCE_FETCH_TAGS", "true")
            .arg("version"),
        "v1.10.0-dev.1.{sha}",
        "v1.9.0",
    );
}

#[test]
fn forced_fetch_reports_missing_origin() {
    TestRepo::new("develop")
        .command()
        .env("FORCE_FETCH_TAGS", "true")
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Failed to retrieve tags"))
        .stderr(predicate::str::contains("origin"));
}
