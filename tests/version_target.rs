mod common;

use assert_cmd::prelude::*;
use common::TestRepo;
use git2::{Oid, Repository, Signature};
use predicates::prelude::*;
use std::process::Command;

fn head(repo: &Repository) -> Oid {
    repo.head().unwrap().target().unwrap()
}

fn commit(repo: &Repository, parent: Oid, message: &str) -> Oid {
    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.find_commit(parent).unwrap();
    repo.commit(
        None,
        &signature,
        &signature,
        message,
        &parent.tree().unwrap(),
        &[&parent],
    )
    .unwrap()
}

fn tag(repo: &Repository, name: &str, id: Oid, annotated: bool) {
    let commit = repo.find_commit(id).unwrap();
    if annotated {
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        repo.tag(name, commit.as_object(), &signature, name, false)
            .unwrap();
    } else {
        repo.tag_lightweight(name, commit.as_object(), false)
            .unwrap();
    }
}

fn released_repo(branch: &str, base: &str, annotated: bool) -> TestRepo {
    let repo = TestRepo::new(branch);
    let base_commit = head(&repo.repo);
    tag(&repo.repo, base, base_commit, annotated);
    let target = commit(&repo.repo, base_commit, "maintenance");
    repo.repo
        .find_reference(&format!("refs/heads/{branch}"))
        .unwrap()
        .set_target(target, "maintenance")
        .unwrap();
    repo
}

fn other_line(repo: &Repository) -> Oid {
    let base = repo.find_commit(head(repo)).unwrap().parent_id(0).unwrap();
    commit(repo, base, "separate release line")
}

fn assert_version(command: &mut Command, upcoming: &str, last: &str) {
    command.assert().success().stdout(format!(
        "UPCOMING_VERSION={upcoming}\nLAST_VERSION={last}\n"
    ));
}

fn assert_error(command: &mut Command, message: &str) {
    command
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(message));
}

#[test]
fn older_release_targets_ignore_newer_official_tags_on_other_branches() {
    for annotated in [false, true] {
        for (branch, target) in [
            ("hotfix/1.2.4", "v1.2.4-rc.1"),
            ("release/1.3.x", "v1.3.0-rc.1"),
            ("release/2.x.x", "v2.0.0-rc.1"),
        ] {
            let repo = released_repo(branch, "v1.2.3", annotated);
            let future = other_line(&repo.repo);
            tag(&repo.repo, "v3.0.0", future, annotated);
            assert_version(repo.command().arg("version"), target, "v1.2.3");
        }
        let repo = released_repo("hotfix/1.2.4", "v1.2.3", annotated);
        tag(&repo.repo, "v2.0.0", other_line(&repo.repo), annotated);
        repo.command()
            .arg("scope")
            .assert()
            .success()
            .stdout("patch\n");
        assert_version(
            repo.command().args(["version", "--scope", "patch"]),
            "v1.2.4-rc.1",
            "v1.2.3",
        );
    }
}

#[test]
fn counters_and_previous_candidates_use_reachable_tags_within_exact_target_and_stage() {
    let repo = released_repo("hotfix/1.2.4", "v1.2.3", true);
    let future = other_line(&repo.repo);
    for name in [
        "v2.0.0",
        "v2.0.1-rc.99",
        "v1.2.5-rc.99",
        "v1.2.4-dev.99.abcd1234",
    ] {
        tag(&repo.repo, name, future, false);
    }
    tag(&repo.repo, "v1.2.4-rc.2", head(&repo.repo), true);
    tag(&repo.repo, "1.2.4-rc.10", future, false);
    assert_version(repo.command().arg("version"), "v1.2.4-rc.3", "v1.2.4-rc.2");
    assert_version(
        repo.command().args(["version", "--stage", "dev"]),
        &format!("v1.2.4-dev.1.{}", &head(&repo.repo).to_string()[..8]),
        "v1.2.3",
    );
    assert_version(
        repo.command().args(["version", "--stage", "stable"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn exact_target_allows_skipped_numbers_and_selects_the_latest_reachable_base_in_line() {
    let repo = released_repo("hotfix/1.2.9", "v1.2.3", false);
    tag(&repo.repo, "v1.3.0", head(&repo.repo), false);
    tag(&repo.repo, "1.2.5", head(&repo.repo), true);
    tag(&repo.repo, "v1.2.8", other_line(&repo.repo), false);
    assert_version(repo.command().arg("version"), "v1.2.9-rc.1", "v1.2.5");
}

#[test]
fn scope_overrides_and_custom_patterns_cannot_change_a_branch_target() {
    for (branch, scope) in [
        ("release/2.x.x", "patch"),
        ("release/1.3.x", "major"),
        ("hotfix/1.2.4", "minor"),
    ] {
        let repo = released_repo(branch, "v1.2.3", false);
        for stage in ["rc", "dev", "stable"] {
            assert_error(
                repo.command()
                    .args(["version", "--scope", scope, "--stage", stage]),
                "conflicts with target",
            );
            assert_error(
                repo.command()
                    .env("SCOPE", scope)
                    .env("STAGE", stage)
                    .arg("version"),
                "conflicts with target",
            );
        }
        assert_error(
            repo.command()
                .env("MAJOR", "^none$")
                .env("MINOR", "^none$")
                .env("PATCH", "^none$")
                .env(scope.to_uppercase(), ".*")
                .arg("version"),
            "conflicts with target",
        );
    }
    let repo = released_repo("release/2.x.x", "v1.2.3", false);
    assert_version(
        repo.command()
            .env("SCOPE", "patch")
            .args(["version", "--scope", "major"]),
        "v2.0.0-rc.1",
        "v1.2.3",
    );
}

#[test]
fn explicit_target_uses_cli_environment_precedence_and_must_agree_with_branch() {
    let repo = released_repo("hotfix/1.2.4", "v1.2.3", false);
    assert_error(
        repo.command().args(["version", "--target", "1.2.5"]),
        "conflicts with branch",
    );
    assert_error(
        repo.command().env("TARGET", "2.0.0").arg("version"),
        "conflicts with branch",
    );
    assert_version(
        repo.command()
            .env("TARGET", "1.2.5")
            .args(["version", "--target", "v1.2.4"]),
        "v1.2.4-rc.1",
        "v1.2.3",
    );
    let repo = released_repo("candidate", "v1.2.3", false);
    tag(&repo.repo, "v2.0.0", other_line(&repo.repo), false);
    assert_version(
        repo.command()
            .env("PATCH", "^candidate$")
            .env("RC", "^candidate$")
            .env("TARGET", "1.2.9")
            .arg("version"),
        "v1.2.9-rc.1",
        "v1.2.3",
    );
    assert_error(
        repo.command().args([
            "version", "--scope", "minor", "--stage", "rc", "--target", "1.2.9",
        ]),
        "conflicts with target",
    );
}

#[test]
fn published_and_non_increasing_targets_fail_in_all_stages() {
    for stage in ["dev", "rc", "stable"] {
        for prefix in ["", "v"] {
            let repo = released_repo("hotfix/1.2.4", "v1.2.3", false);
            tag(
                &repo.repo,
                &format!("{prefix}1.2.4"),
                other_line(&repo.repo),
                true,
            );
            assert_error(
                repo.command().args(["version", "--stage", stage]),
                "already released",
            );
        }
        for (branch, base) in [
            ("hotfix/1.2.4", "v1.2.5"),
            ("release/1.3.x", "v1.3.2"),
            ("release/2.x.x", "v2.0.0"),
        ] {
            let repo = released_repo(branch, base, false);
            assert_error(
                repo.command().args(["version", "--stage", stage]),
                "must be newer",
            );
        }
    }
}

#[test]
fn missing_release_line_base_does_not_fall_back_to_an_unrelated_line_or_zero() {
    for (branch, base) in [
        ("hotfix/1.2.4", "v1.3.0"),
        ("release/1.3.x", "v2.0.0"),
        ("hotfix/0.0.1", "v1.0.0"),
    ] {
        let repo = released_repo(branch, base, false);
        tag(&repo.repo, "v1.2.3", other_line(&repo.repo), false);
        assert_error(repo.command().arg("version"), "No reachable official base");
    }
    for branch in ["hotfix/1.2.4", "release/1.3.x"] {
        assert_error(
            TestRepo::new(branch).command().arg("version"),
            "No reachable official base",
        );
    }
}

#[test]
fn unsupported_branches_and_target_formats_fail_even_with_explicit_policy() {
    for branch in [
        "hotfix/1a2b4",
        "hotfix/v1.2.4",
        "hotfix/01.2.4",
        "hotfix/1.2.0",
        "release/1.3.0",
        "release/1.0.x",
        "release/0.x.x",
        "release/1a3bx",
        "release/1.3.x/extra",
        "release/18446744073709551616.x.x",
    ] {
        assert_error(
            TestRepo::new(branch)
                .command()
                .args(["version", "--scope", "patch", "--stage", "rc"]),
            "Unsupported release target branch",
        );
    }
    let repo = TestRepo::new("custom");
    for target in [
        "1.2.x",
        "1.2.3-rc.1",
        "1.2.3+build",
        "01.2.3",
        "1.2.3.4",
        "0.0.0",
        "18446744073709551616.0.0",
    ] {
        assert_error(
            repo.command().args([
                "version", "--scope", "patch", "--stage", "rc", "--target", target,
            ]),
            "Invalid target",
        );
    }
}

#[test]
fn target_promotion_requires_a_matching_candidate_and_never_falls_back() {
    let repo = released_repo("hotfix/1.2.4", "v1.2.3", false);
    tag(&repo.repo, "v2.0.0-rc.1", head(&repo.repo), false);
    let future = other_line(&repo.repo);
    tag(&repo.repo, "v1.2.4-rc.1", future, false);
    assert_error(
        repo.command().args(["version", "--scope", "release"]),
        "No reachable prerelease for target v1.2.4",
    );
    assert_error(
        repo.command()
            .args(["version", "--stage", "stable", "--candidate", "v2.0.0-rc.1"]),
        "conflicts with target",
    );
    assert_version(
        repo.command()
            .args(["version", "--stage", "stable", "--candidate", "v1.2.4-rc.1"]),
        "v1.2.4",
        "v1.2.3",
    );
    tag(&repo.repo, "v1.2.4-rc.2", head(&repo.repo), true);
    assert_version(
        repo.command().args(["version", "--scope", "release"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn targeted_prereleases_require_complete_history() {
    let repo = released_repo("hotfix/1.2.4", "v1.2.3", false);
    std::fs::write(
        repo.repo.path().join("shallow"),
        format!("{}\n", head(&repo.repo)),
    )
    .unwrap();
    assert_error(repo.command().arg("version"), "requires complete history");
}
