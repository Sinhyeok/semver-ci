mod common;

use assert_cmd::prelude::*;
use common::TestRepo;
use predicates::prelude::*;

#[test]
fn scope_matches_each_git_flow_branch() {
    for (branch, scope) in [
        ("develop", "minor"),
        ("feature/nested/topic", "minor"),
        ("release/2.x.x", "major"),
        ("release/1.2.x", "minor"),
        ("hotfix/1.2.3", "patch"),
        ("main", "release"),
        ("master", "release"),
    ] {
        TestRepo::new(branch)
            .command()
            .arg("scope")
            .assert()
            .success()
            .stdout(format!("{scope}\n"));
    }
}

#[test]
fn scope_rejects_unknown_and_partial_branch_matches() {
    for branch in [
        "bugfix/example",
        "develop-extra",
        "prefix/main",
        "release/1.2.3",
    ] {
        TestRepo::new(branch)
            .command()
            .arg("scope")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(format!(
                "Unknown branch name: {branch}"
            )));
    }
}

#[test]
fn scope_accepts_custom_patterns_from_flags_and_environment() {
    let repo = TestRepo::new("custom/topic");
    for (scope, variable) in [
        ("major", "MAJOR"),
        ("minor", "MINOR"),
        ("patch", "PATCH"),
        ("release", "RELEASE"),
    ] {
        repo.command()
            .args(["scope", &format!("--{scope}"), "^custom/.*$"])
            .assert()
            .success()
            .stdout(format!("{scope}\n"));
        repo.command()
            .env(variable, "^custom/.*$")
            .arg("scope")
            .assert()
            .success()
            .stdout(format!("{scope}\n"));
    }
}

#[test]
fn scope_flag_overrides_environment_pattern() {
    TestRepo::new("develop")
        .command()
        .env("MAJOR", "^develop$")
        .args(["scope", "--major", "^custom$"])
        .assert()
        .success()
        .stdout("minor\n");
}

#[test]
fn scope_release_pattern_ignores_stable_and_keeps_legacy_overrides() {
    let repo = TestRepo::new("production");
    repo.command()
        .env("STABLE", "^production$")
        .arg("scope")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Unknown branch name: production"));
    repo.command()
        .env("STABLE", "^production$")
        .env("RELEASE", "^other$")
        .arg("scope")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Unknown branch name: production"));
    repo.command()
        .env("STABLE", "^other$")
        .env("RELEASE", "^production$")
        .arg("scope")
        .assert()
        .success()
        .stdout("release\n");
    repo.command()
        .env("STABLE", "^production$")
        .env("RELEASE", "^other$")
        .args(["scope", "--release", "^production$"])
        .assert()
        .success()
        .stdout("release\n");
    TestRepo::new("main")
        .command()
        .env("STABLE", "[")
        .arg("scope")
        .assert()
        .success()
        .stdout("release\n");
}

#[test]
fn scope_uses_major_minor_patch_release_precedence() {
    let repo = TestRepo::new("custom");
    let mut args = vec!["scope", "--release", ".*"];
    for (flag, expected) in [
        ("--patch", "patch"),
        ("--minor", "minor"),
        ("--major", "major"),
    ] {
        args.extend([flag, ".*"]);
        repo.command()
            .args(&args)
            .assert()
            .success()
            .stdout(format!("{expected}\n"));
    }
}

#[test]
fn scope_reports_invalid_regex() {
    let repo = TestRepo::new("develop");
    for flag in ["--major", "--minor", "--patch", "--release"] {
        repo.command()
            .args(["scope", flag, "["])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("regex parse error"));
    }
}

#[test]
fn scope_reports_detached_head() {
    let repo = TestRepo::new("develop");
    let head = repo.repo.head().unwrap().target().unwrap();
    repo.repo.set_head_detached(head).unwrap();
    repo.command()
        .arg("scope")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("HEAD is in detached state"));
}

#[test]
fn scope_uses_ci_branch_names_and_prefers_github_when_both_are_enabled() {
    let repo = TestRepo::new("develop");
    repo.command()
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_REF_NAME", "main")
        .env("GITLAB_CI", "true")
        .env("CI_COMMIT_REF_NAME", "hotfix/1.2.3")
        .arg("scope")
        .assert()
        .success()
        .stdout("release\n");
    repo.command()
        .env("GITLAB_CI", "true")
        .env("CI_COMMIT_REF_NAME", "hotfix/1.2.3")
        .arg("scope")
        .assert()
        .success()
        .stdout("patch\n");
}
