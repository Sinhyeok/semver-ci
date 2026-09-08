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
    let sha = repo.repo.head().unwrap().target().unwrap().to_string();
    let upcoming = upcoming.replace("{sha}", &sha[..8]);
    command.assert().success().stdout(format!(
        "UPCOMING_VERSION={upcoming}\nLAST_VERSION={last}\n"
    ));
}

#[test]
fn version_alone_infers_all_default_scope_and_stage_pairs() {
    for (branch, version) in [
        ("develop", "v1.3.0-dev.1.{sha}"),
        ("feature/nested/topic", "v1.3.0-dev.1.{sha}"),
        ("release/2.x.x", "v2.0.0-rc.1"),
        ("release/1.3.x", "v1.3.0-rc.1"),
        ("hotfix/1.2.4", "v1.2.4-rc.1"),
        ("main", "v1.3.0"),
        ("master", "v1.3.0"),
    ] {
        let repo = repo_with_tags(branch, &["v1.2.3"]);
        assert_version(&repo, repo.command().arg("version"), version, "v1.2.3");
    }
}

#[test]
fn each_bump_scope_works_with_each_stage_including_stable() {
    for (scope, version) in [("major", "2.0.0"), ("minor", "1.3.0"), ("patch", "1.2.4")] {
        for (stage, suffix) in [("dev", "-dev.1.{sha}"), ("rc", "-rc.1"), ("stable", "")] {
            // These candidates must neither override a bump nor cause ambiguity.
            let repo = repo_with_tags("main", &["v1.2.3", "v3.0.0-rc.1", "v4.0.0-rc.1"]);
            assert_version(
                &repo,
                repo.command()
                    .args(["version", "--scope", scope, "--stage", stage]),
                &format!("v{version}{suffix}"),
                "v1.2.3",
            );
        }
    }
}

#[test]
fn only_the_missing_value_is_inferred_from_the_branch() {
    for (branch, args, version) in [
        ("release/2.x.x", ["--stage", "dev"], "v2.0.0-dev.1.{sha}"),
        ("develop", ["--scope", "patch"], "v1.2.4-dev.1.{sha}"),
        ("hotfix/1.2.4", ["--stage", "stable"], "v1.2.4"),
        ("main", ["--scope", "patch"], "v1.2.4"),
    ] {
        let repo = repo_with_tags(branch, &["v1.2.3"]);
        assert_version(
            &repo,
            repo.command().arg("version").args(args),
            version,
            "v1.2.3",
        );
    }
}

#[test]
fn cli_overrides_environment_which_overrides_branch_rules_for_each_value() {
    let repo = repo_with_tags("release/2.x.x", &["v1.2.3"]);
    for (args, version) in [
        (vec![], "v1.2.4-rc.1"),
        (vec!["--scope", "minor"], "v1.3.0-rc.1"),
        (vec!["--stage", "dev"], "v1.2.4-dev.1.{sha}"),
        (vec!["--scope", "major", "--stage", "stable"], "v2.0.0"),
    ] {
        assert_version(
            &repo,
            repo.command()
                .env("SCOPE", "patch")
                .env("STAGE", "rc")
                .arg("version")
                .args(args),
            version,
            "v1.2.3",
        );
    }
}

#[test]
fn custom_scope_and_stage_patterns_are_read_from_environment() {
    for (scope_variable, stage_variable, version, last) in [
        ("MAJOR", "DEV", "v2.0.0-dev.1.{sha}", "v1.2.3"),
        ("MINOR", "DEV", "v1.3.0-dev.1.{sha}", "v1.2.3"),
        ("PATCH", "RC", "v1.2.4-rc.2", "v1.2.4-rc.1"),
        ("MINOR", "STABLE", "v1.3.0", "v1.2.3"),
        ("RELEASE", "STABLE", "v1.2.4", "v1.2.3"),
    ] {
        let repo = repo_with_tags("integration", &["v1.2.3", "v1.2.4-rc.1"]);
        assert_version(
            &repo,
            repo.command()
                .env(scope_variable, "^integration$")
                .env(stage_variable, "^integration$")
                .arg("version"),
            version,
            last,
        );
    }
}

#[test]
fn release_scope_pattern_is_independent_of_the_stable_environment() {
    let repo = repo_with_tags("production", &["v1.2.3", "v1.2.4-rc.1"]);
    repo.command()
        .env("STABLE", "^production$")
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--scope"));
    assert_version(
        &repo,
        repo.command()
            .env("RELEASE", "^production$")
            .env("STABLE", "^production$")
            .arg("version"),
        "v1.2.4",
        "v1.2.3",
    );
    repo.command()
        .env("RELEASE", "^other$")
        .env("STABLE", "^production$")
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--scope"));

    // Stage is explicit, so changing STABLE must not move the default release scope.
    let main = repo_with_tags("main", &["v1.2.3", "v1.2.4-rc.1"]);
    assert_version(
        &main,
        main.command()
            .env("STABLE", "^production$")
            .args(["version", "--stage", "stable"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn stable_stage_does_not_inherit_the_legacy_release_pattern() {
    let repo = repo_with_tags("production", &["v1.2.3", "v1.2.4-rc.1"]);
    repo.command()
        .env("RELEASE", "^production$")
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--stage"));

    // The existing two-command workflow still explicitly selects stable promotion.
    let output = repo
        .command()
        .env("RELEASE", "^production$")
        .arg("scope")
        .assert()
        .success();
    let scope = std::str::from_utf8(&output.get_output().stdout)
        .unwrap()
        .trim();
    assert_eq!(scope, "release");
    assert_version(
        &repo,
        repo.command()
            .env("RELEASE", "^production$")
            .env("SCOPE", scope)
            .arg("version"),
        "v1.2.4",
        "v1.2.3",
    );

    // An unused RELEASE pattern must not affect default stable stage inference.
    let main = repo_with_tags("main", &["v1.2.3"]);
    assert_version(
        &main,
        main.command()
            .env("RELEASE", "[")
            .args(["version", "--scope", "patch"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn branch_pattern_precedence_is_deterministic_and_shared_with_scope() {
    let repo = repo_with_tags("custom", &["v1.2.3"]);
    let mut patterns = vec![("RELEASE", ".*")];
    for (variable, scope, version) in [
        ("PATCH", "patch", "v1.2.4-dev.1.{sha}"),
        ("MINOR", "minor", "v1.3.0-dev.1.{sha}"),
        ("MAJOR", "major", "v2.0.0-dev.1.{sha}"),
    ] {
        patterns.push((variable, ".*"));
        repo.command()
            .envs(patterns.clone())
            .arg("scope")
            .assert()
            .success()
            .stdout(format!("{scope}\n"));
        assert_version(
            &repo,
            repo.command()
                .envs(patterns.clone())
                .env("DEV", ".*")
                .env("RC", ".*")
                .env("STABLE", ".*")
                .arg("version"),
            version,
            "v1.2.3",
        );
    }
    assert_version(
        &repo,
        repo.command()
            .env("SCOPE", "patch")
            .env("RC", ".*")
            .env("STABLE", ".*")
            .arg("version"),
        "v1.2.4-rc.1",
        "v1.2.3",
    );
}

#[test]
fn unknown_branches_require_every_missing_value_to_be_configured() {
    let repo = repo_with_tags("integration", &["v1.2.3"]);
    for (args, missing) in [
        (vec!["--scope", "minor"], "--stage"),
        (vec!["--stage", "dev"], "--scope"),
    ] {
        repo.command()
            .arg("version")
            .args(args)
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("integration"))
            .stderr(predicate::str::contains(missing));
    }
    repo.command()
        .env("MINOR", "^integration$")
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--stage"));
    for branch in ["integration", "develop-extra", "prefix/main"] {
        TestRepo::new(branch)
            .command()
            .arg("version")
            .assert()
            .failure()
            .stdout("");
    }
    assert_version(
        &repo,
        repo.command()
            .args(["version", "--scope", "minor", "--stage", "dev"]),
        "v1.3.0-dev.1.{sha}",
        "v1.2.3",
    );
}

#[test]
fn explicit_values_do_not_need_branch_patterns() {
    let repo = repo_with_tags("integration", &["v1.2.3"]);
    let mut command = repo.command();
    for variable in ["MAJOR", "MINOR", "PATCH", "RELEASE", "DEV", "RC", "STABLE"] {
        command.env(variable, "[");
    }
    assert_version(
        &repo,
        command.args(["version", "--scope", "patch", "--stage", "stable"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn invalid_patterns_needed_for_inference_fail_without_version_outputs() {
    let repo = TestRepo::new("develop");
    for variable in ["MAJOR", "MINOR", "PATCH", "RELEASE", "DEV", "RC", "STABLE"] {
        repo.command()
            .env(variable, "[")
            .arg("version")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("regex parse error"));
    }
}

#[test]
fn release_scope_is_compatible_but_cannot_be_combined_with_a_prerelease_stage() {
    let repo = repo_with_tags("develop", &["v1.2.3", "v1.2.4-rc.1"]);
    assert_version(
        &repo,
        repo.command().env("SCOPE", "release").arg("version"),
        "v1.2.4",
        "v1.2.3",
    );
    assert_version(
        &repo,
        repo.command()
            .env("DEV", "[")
            .args(["version", "--scope", "release"]),
        "v1.2.4",
        "v1.2.3",
    );
    for stage in ["dev", "rc"] {
        repo.command()
            .args(["version", "--scope", "release", "--stage", stage])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("release requires stage stable"));
        repo.command()
            .env("SCOPE", "release")
            .env("STAGE", stage)
            .arg("version")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("release requires stage stable"));
    }
    TestRepo::new("main")
        .command()
        .args(["version", "--stage", "rc"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("release requires stage stable"));
}

#[test]
fn invalid_stage_values_are_rejected_instead_of_becoming_stable() {
    for value in ["release", "unknown", ""] {
        TestRepo::new("main")
            .command()
            .args(["version", "--stage", value])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("--stage <STAGE>"))
            .stderr(predicate::str::contains("possible values: dev, rc, stable"));
    }
    TestRepo::new("main")
        .command()
        .env("STAGE", "release")
        .arg("version")
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn explicit_candidate_defaults_scope_to_release_even_on_a_bump_branch() {
    let repo = repo_with_tags("develop", &["v1.2.3", "v1.2.4-rc.1", "v2.0.0-rc.1"]);
    assert_version(
        &repo,
        repo.command()
            .args(["version", "--stage", "stable", "--candidate", "v1.2.4-rc.1"]),
        "v1.2.4",
        "v1.2.3",
    );
    assert_version(
        &repo,
        repo.command()
            .env("STAGE", "stable")
            .env("CANDIDATE", "v1.2.4-rc.1")
            .arg("version"),
        "v1.2.4",
        "v1.2.3",
    );
    let custom = repo_with_tags("integration", &["v1.2.3", "v1.2.4-rc.1"]);
    assert_version(
        &custom,
        custom
            .command()
            .args(["version", "--stage", "stable", "--candidate", "v1.2.4-rc.1"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn explicit_candidate_conflicts_with_an_explicit_bump_scope() {
    let repo = repo_with_tags("main", &["v1.2.3", "v1.2.4-rc.1"]);
    for scope in ["major", "minor", "patch"] {
        repo.command()
            .args(["version", "--scope", scope, "--candidate", "v1.2.4-rc.1"])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("--candidate cannot be combined"));
        repo.command()
            .env("SCOPE", scope)
            .env("CANDIDATE", "v1.2.4-rc.1")
            .arg("version")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("--candidate cannot be combined"));
    }
}

#[test]
fn stable_release_keeps_the_minor_fallback_and_initial_version() {
    let repo = repo_with_tags("main", &["v1.2.3"]);
    assert_version(
        &repo,
        repo.command().args(["version", "--stage", "stable"]),
        "v1.3.0",
        "v1.2.3",
    );
    let initial = TestRepo::new("main");
    assert_version(
        &initial,
        initial.command().arg("version"),
        "v0.1.0",
        "v0.0.0",
    );
}

#[test]
fn legacy_scope_output_can_still_be_passed_to_version_unchanged() {
    for (branch, version) in [
        ("develop", "v1.3.0-dev.1.{sha}"),
        ("release/2.x.x", "v2.0.0-rc.1"),
        ("hotfix/1.2.4", "v1.2.4-rc.2"),
        ("main", "v1.2.4"),
        ("master", "v1.2.4"),
    ] {
        let repo = repo_with_tags(branch, &["v1.2.3", "v1.2.4-rc.1"]);
        let result = repo
            .command()
            .env("DEV", "[")
            .arg("scope")
            .assert()
            .success();
        let scope = std::str::from_utf8(&result.get_output().stdout)
            .unwrap()
            .trim();
        let last = if branch.starts_with("hotfix/") {
            "v1.2.4-rc.1"
        } else {
            "v1.2.3"
        };
        assert_version(
            &repo,
            repo.command().env("SCOPE", scope).arg("version"),
            version,
            last,
        );
    }
}
