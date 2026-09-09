mod common;

use assert_cmd::prelude::*;
use common::TestRepo;
use git2::{Oid, Repository, Signature};
use predicates::prelude::*;
use std::process::Command;

fn head(repo: &Repository) -> Oid {
    repo.head().unwrap().peel_to_commit().unwrap().id()
}

fn commit(repo: &Repository, branch: &str, message: &str, parents: &[Oid]) -> Oid {
    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let parents: Vec<_> = parents
        .iter()
        .map(|id| repo.find_commit(*id).unwrap())
        .collect();
    let tree = parents[0].tree().unwrap();
    repo.commit(
        Some(&format!("refs/heads/{branch}")),
        &signature,
        &signature,
        message,
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

fn tag(repo: &Repository, name: &str, commit: Oid, annotated: bool) {
    let object = repo.find_commit(commit).unwrap();
    if annotated {
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        repo.tag(name, object.as_object(), &signature, name, false)
            .unwrap();
    } else {
        repo.tag_lightweight(name, object.as_object(), false)
            .unwrap();
    }
}

fn released_repo() -> TestRepo {
    let repo = TestRepo::new("main");
    tag(&repo.repo, "v1.2.3", head(&repo.repo), true);
    repo
}

fn assert_official(command: &mut Command, upcoming: &str, last: &str) {
    command.assert().success().stdout(format!(
        "UPCOMING_VERSION={upcoming}\nLAST_VERSION={last}\n"
    ));
}

#[test]
fn promotes_merged_hotfix_and_ignores_unrelated_prerelease_and_official_tags() {
    for annotated in [false, true] {
        let repo = released_repo();
        let base = head(&repo.repo);
        let hotfix = commit(&repo.repo, "hotfix/1.2.4", "fix: hotfix", &[base]);
        tag(&repo.repo, "v1.2.4-rc.1", hotfix, annotated);
        let future = commit(&repo.repo, "release/2.x.x", "feat: future release", &[base]);
        tag(&repo.repo, "v2.0.0-rc.1", future, annotated);
        tag(&repo.repo, "v1.9.0", future, annotated);
        commit(&repo.repo, "main", "Merge hotfix", &[base, hotfix]);

        assert_official(
            repo.command().args(["version", "--scope", "release"]),
            "v1.2.4",
            "v1.2.3",
        );
    }
}

#[test]
fn promotes_candidate_at_target_after_fast_forward() {
    let repo = released_repo();
    let candidate = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
    tag(&repo.repo, "1.2.4-rc.1", candidate, false);
    assert_official(repo.command().arg("version"), "v1.2.4", "v1.2.3");
}

#[test]
fn unrelated_candidates_do_not_change_the_rc_free_minor_fallback() {
    let repo = released_repo();
    let base = head(&repo.repo);
    let future = commit(&repo.repo, "release/2.x.x", "feat: future release", &[base]);
    tag(&repo.repo, "v2.0.0-rc.1", future, false);
    commit(&repo.repo, "main", "feat: direct change", &[base]);
    assert_official(repo.command().arg("version"), "v1.3.0", "v1.2.3");
}

fn assert_stable_collision(scope: &str, candidate: Option<&str>, upcoming: &str) {
    for annotated in [false, true] {
        for prefix in ["", "v"] {
            let repo = released_repo();
            let base = head(&repo.repo);
            let target = commit(&repo.repo, "main", "current work", &[base]);
            if let Some(candidate) = candidate {
                tag(&repo.repo, candidate, target, annotated);
            }
            let future = commit(&repo.repo, "future", "separate release", &[base]);
            let published = format!("{prefix}{upcoming}");
            tag(&repo.repo, &published, future, annotated);

            repo.command()
                .args(["version", "--scope", scope, "--stage", "stable"])
                .assert()
                .code(1)
                .stdout("")
                .stderr(predicate::str::contains(format!(
                    "Upcoming version v{upcoming} already exists as tag '{published}'"
                )));
        }
    }
}

#[test]
fn automatic_promotion_rejects_an_upcoming_version_published_outside_target_history() {
    assert_stable_collision("release", Some("v1.2.4-rc.1"), "1.2.4");
}

#[test]
fn direct_stable_bumps_reject_an_upcoming_version_published_outside_target_history() {
    for (scope, upcoming) in [("patch", "1.2.4"), ("minor", "1.3.0"), ("major", "2.0.0")] {
        assert_stable_collision(scope, None, upcoming);
    }
}

#[test]
fn minor_fallback_rejects_an_upcoming_version_published_outside_target_history() {
    assert_stable_collision("release", None, "1.3.0");
}

#[test]
fn prereleases_reject_an_upcoming_version_published_outside_target_history() {
    for annotated in [false, true] {
        for prefix in ["", "v"] {
            for stage in ["dev", "rc"] {
                let repo = released_repo();
                let base = head(&repo.repo);
                let target = commit(&repo.repo, "main", "current work", &[base]);
                let future = commit(&repo.repo, "future", "separate release", &[base]);
                let suffix = if stage == "dev" {
                    format!(".{}", &target.to_string()[..8])
                } else {
                    String::new()
                };
                let upcoming = format!("1.3.0-{stage}.1{suffix}");
                let published = format!("{prefix}{upcoming}");
                tag(&repo.repo, &published, future, annotated);

                for exact_target in [false, true] {
                    let mut command = repo.command();
                    command.args(["version", "--scope", "minor", "--stage", stage]);
                    if exact_target {
                        command.args(["--target", "1.3.0"]);
                    }
                    command
                        .assert()
                        .code(1)
                        .stdout("")
                        .stderr(predicate::str::contains(format!(
                            "Upcoming version v{upcoming} already exists as tag '{published}'"
                        )));
                }
            }
        }
    }
}

#[test]
fn upcoming_version_uniqueness_compares_the_full_prerelease_including_dev_sha() {
    let repo = released_repo();
    let base = head(&repo.repo);
    let target = commit(&repo.repo, "main", "current work", &[base]);
    let future = commit(&repo.repo, "future", "separate release", &[base]);
    let target_sha = target.to_string()[..8].to_string();
    let other_sha = if target_sha == "aaaaaaaa" {
        "bbbbbbbb"
    } else {
        "aaaaaaaa"
    };
    for name in [
        format!("1.3.0-dev.1.{other_sha}"),
        format!("v1.3.0-dev.1.{target_sha}.extra"),
        "v1.3.0-rc.1.extra".to_string(),
    ] {
        tag(&repo.repo, &name, future, false);
    }

    for (stage, upcoming) in [
        ("dev", format!("v1.3.0-dev.1.{target_sha}")),
        ("rc", "v1.3.0-rc.1".to_string()),
    ] {
        assert_official(
            repo.command()
                .args(["version", "--scope", "minor", "--stage", stage]),
            &upcoming,
            "v1.2.3",
        );
    }
}

#[test]
fn shallow_history_is_an_error_even_when_tags_are_available() {
    let repo = released_repo();
    let target = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
    tag(&repo.repo, "v1.2.4-rc.1", target, false);
    // Git's shallow boundary truncates ancestry even if older objects exist locally.
    std::fs::write(repo.repo.path().join("shallow"), format!("{target}\n")).unwrap();
    assert!(repo.repo.is_shallow());
    repo.command()
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("requires complete history"))
        .stderr(predicate::str::contains("git fetch --unshallow --tags"));
}

fn ci_command(repo: &TestRepo, github: bool, target: &str) -> Command {
    // Both integrations fetch tags; use a local origin and isolated Git config.
    // libgit2 discovers global config through XDG_CONFIG_HOME, not GIT_CONFIG_GLOBAL.
    let config_root = repo.repo.workdir().unwrap().join("test-config");
    std::fs::create_dir_all(config_root.join("git")).unwrap();
    std::fs::write(config_root.join("git/config"), "").unwrap();
    let mut command = repo.command();
    command.env("XDG_CONFIG_HOME", &config_root);
    if github {
        command
            .env("GITHUB_ACTIONS", "true")
            .env("GITHUB_REF_NAME", "main")
            .env("GITHUB_SHA", target)
            .env("GITHUB_ACTOR", "test-user")
            .env("GITHUB_TOKEN", "test-token");
    } else {
        command
            .env("GITLAB_CI", "true")
            .env("CI_COMMIT_REF_NAME", "main")
            .env("CI_COMMIT_SHA", target)
            .env("CI_COMMIT_SHORT_SHA", &target[..8])
            .env("GITLAB_USER_EMAIL", "test@example.com")
            .env("CI_JOB_TOKEN", "test-token")
            .env("CI_PROJECT_URL", "https://example.invalid/project");
    }
    command.arg("version");
    command
}

#[cfg(unix)]
#[path = "common/git_server.rs"]
mod git_server;

#[cfg(unix)]
mod shallow_ci {
    use super::*;

    fn shallow_clone(url: &str) -> (tempfile::TempDir, Repository) {
        let directory = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["clone", "--depth=1", "--no-tags", url])
            .arg(directory.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .assert()
            .success();
        let repo = Repository::open(directory.path()).unwrap();
        assert!(repo.is_shallow());
        assert_eq!(repo.tag_names(None).unwrap().len(), 0);
        (directory, repo)
    }

    #[test]
    fn ci_completes_history_without_changing_the_checkout_or_event_commit() {
        let source = released_repo();
        let candidate = commit(&source.repo, "main", "candidate", &[head(&source.repo)]);
        tag(&source.repo, "v1.2.4-rc.2", candidate, true);
        tag(&source.repo, "v1.2.4-dev.3.abcdef12", candidate, false);
        for index in 0..25 {
            commit(
                &source.repo,
                "main",
                &format!("work {index}"),
                &[head(&source.repo)],
            );
        }
        let event = head(&source.repo);
        let newer = commit(&source.repo, "main", "newer checkout", &[event]);
        tag(&source.repo, "v2.0.0", newer, true);
        let server = git_server::GitServer::new(source.repo.workdir().unwrap());

        for github in [false, true] {
            for (stage, scope, upcoming, last) in [
                ("stable", "release", "v1.2.4".to_string(), "v1.2.3"),
                ("rc", "patch", "v1.2.4-rc.3".to_string(), "v1.2.4-rc.2"),
                (
                    "dev",
                    "patch",
                    format!("v1.2.4-dev.4.{}", &event.to_string()[..8]),
                    "v1.2.4-dev.3.abcdef12",
                ),
            ] {
                let (directory, checkout) = shallow_clone(&server.url());
                assert!(checkout.find_commit(candidate).is_err());
                checkout.set_head_detached(newer).unwrap();
                std::fs::write(directory.path().join("keep.txt"), "local work").unwrap();
                let mut command = ci_command(&source, github, &event.to_string());
                command
                    .env("CLONE_TARGET_PATH", directory.path())
                    .args(["--stage", stage, "--scope", scope]);
                assert_official(&mut command, &upcoming, last);
                let checkout = Repository::open(directory.path()).unwrap();
                assert!(!checkout.is_shallow());
                assert_eq!(head(&checkout), newer);
                assert!(checkout.head_detached().unwrap());
                assert_eq!(
                    std::fs::read_to_string(directory.path().join("keep.txt")).unwrap(),
                    "local work"
                );
                // A later invocation still fetches tags but does not unshallow again.
                command
                    .assert()
                    .success()
                    .stderr(predicate::str::contains("Shallow CI checkout").not());
            }
        }
    }

    #[test]
    fn ci_fetches_tags_outside_the_event_history_to_detect_collisions() {
        let source = released_repo();
        let base = head(&source.repo);
        let target = commit(&source.repo, "main", "candidate", &[base]);
        tag(&source.repo, "v1.2.4-rc.1", target, false);
        let other = commit(&source.repo, "other", "separate release", &[base]);
        tag(&source.repo, "v1.2.4", other, true);
        let server = git_server::GitServer::new(source.repo.workdir().unwrap());
        for github in [false, true] {
            let (directory, _) = shallow_clone(&server.url());
            ci_command(&source, github, &target.to_string())
                .env("CLONE_TARGET_PATH", directory.path())
                .assert()
                .failure()
                .stdout("")
                .stderr(predicate::str::contains("already exists as tag 'v1.2.4'"));
        }
    }

    #[test]
    fn ci_fetch_failure_has_no_version_output_and_local_runs_do_not_fetch_history() {
        let source = released_repo();
        let target = commit(&source.repo, "main", "current work", &[head(&source.repo)]);
        let server = git_server::GitServer::new(source.repo.workdir().unwrap());
        for github in [false, true] {
            let (directory, checkout) = shallow_clone(&server.url());
            checkout
                .remote_set_url(
                    "origin",
                    directory.path().join("missing.git").to_str().unwrap(),
                )
                .unwrap();
            ci_command(&source, github, &target.to_string())
                .env("CLONE_TARGET_PATH", directory.path())
                .assert()
                .failure()
                .stdout("")
                .stderr(predicate::str::contains(
                    "Failed to fetch complete CI history from origin",
                ));
            source
                .command()
                .arg("version")
                .env("CLONE_TARGET_PATH", directory.path())
                .assert()
                .failure()
                .stdout("")
                .stderr(predicate::str::contains("requires complete history"))
                .stderr(predicate::str::contains("Shallow CI checkout").not());
            assert!(Repository::open(directory.path()).unwrap().is_shallow());
        }
    }

    #[test]
    fn ci_rejects_a_remote_that_cannot_supply_complete_history() {
        let source = released_repo();
        let target = commit(&source.repo, "main", "current work", &[head(&source.repo)]);
        let server = git_server::GitServer::new(source.repo.workdir().unwrap());
        let (origin_directory, _) = shallow_clone(&server.url());
        let shallow_origin = git_server::GitServer::new(origin_directory.path());
        for github in [false, true] {
            let (directory, _) = shallow_clone(&shallow_origin.url());
            ci_command(&source, github, &target.to_string())
                .env("CLONE_TARGET_PATH", directory.path())
                .assert()
                .failure()
                .stdout("")
                .stderr(
                    predicate::str::contains("Failed to fetch complete CI history").or(
                        predicate::str::contains("Repository is still shallow after fetching"),
                    ),
                );
            assert!(Repository::open(directory.path()).unwrap().is_shallow());
        }
    }
}

#[test]
fn both_ci_providers_infer_policy_from_the_ci_branch_without_scope_command() {
    for github in [false, true] {
        let repo = released_repo();
        repo.repo
            .remote("origin", repo.repo.path().to_str().unwrap())
            .unwrap();
        let target = head(&repo.repo).to_string();
        let branch_variable = if github {
            "GITHUB_REF_NAME"
        } else {
            "CI_COMMIT_REF_NAME"
        };
        for (branch, upcoming) in [
            ("release/2.x.x", "v2.0.0-rc.1"),
            ("hotfix/1.2.4", "v1.2.4-rc.1"),
        ] {
            assert_official(
                ci_command(&repo, github, &target).env(branch_variable, branch),
                upcoming,
                "v1.2.3",
            );
        }
        assert_official(
            ci_command(&repo, github, &target)
                .env(branch_variable, "integration")
                .env("MINOR", "^integration$")
                .env("DEV", "^integration$"),
            &format!("v1.3.0-dev.1.{}", &target[..8]),
            "v1.2.3",
        );
    }
}

#[test]
fn ci_uses_the_event_history_and_branch_target_for_prereleases_and_stable_bumps() {
    for github in [false, true] {
        let repo = released_repo();
        let base = head(&repo.repo);
        let target = commit(&repo.repo, "main", "maintenance event", &[base]);
        tag(&repo.repo, "v1.2.4-rc.2", target, true);
        let newer = commit(&repo.repo, "main", "newer checkout", &[target]);
        tag(&repo.repo, "v1.2.9", newer, false);
        let future = commit(&repo.repo, "release/2.x.x", "separate major", &[base]);
        tag(&repo.repo, "v2.0.0", future, true);
        repo.repo
            .remote("origin", repo.repo.path().to_str().unwrap())
            .unwrap();
        let branch_variable = if github {
            "GITHUB_REF_NAME"
        } else {
            "CI_COMMIT_REF_NAME"
        };
        assert_official(
            ci_command(&repo, github, &target.to_string()).env(branch_variable, "hotfix/1.2.4"),
            "v1.2.4-rc.3",
            "v1.2.4-rc.2",
        );
        assert_official(
            ci_command(&repo, github, &target.to_string())
                .env(branch_variable, "hotfix/1.2.4")
                .args(["--stage", "stable"]),
            "v1.2.4",
            "v1.2.3",
        );
        ci_command(&repo, github, "0000000000000000000000000000000000000001")
            .env(branch_variable, "hotfix/1.2.4")
            .assert()
            .failure()
            .stdout("");
    }
}

#[test]
fn stable_bump_uses_the_targets_official_history_without_promoting_candidates() {
    let repo = released_repo();
    let base = head(&repo.repo);
    let future = commit(&repo.repo, "release/9.x.x", "feat: future", &[base]);
    tag(&repo.repo, "v9.0.0", future, false);
    tag(&repo.repo, "v9.1.0-rc.1", future, false);
    let target = commit(&repo.repo, "main", "fix: target", &[base]);
    tag(&repo.repo, "v2.0.0-rc.1", target, true);
    assert_official(
        repo.command()
            .args(["version", "--scope", "patch", "--stage", "stable"]),
        "v1.2.4",
        "v1.2.3",
    );
    std::fs::write(repo.repo.path().join("shallow"), format!("{target}\n")).unwrap();
    repo.command()
        .args(["version", "--scope", "patch", "--stage", "stable"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("requires complete history"));
}

#[test]
fn ci_uses_the_event_commit_instead_of_a_newer_checkout() {
    for github in [false, true] {
        let repo = released_repo();
        let target = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
        tag(&repo.repo, "v1.2.4-rc.1", target, false);
        let newer = commit(&repo.repo, "main", "feat: newer release", &[target]);
        tag(&repo.repo, "v2.0.0-rc.1", newer, false);
        repo.repo
            .remote("origin", repo.repo.path().to_str().unwrap())
            .unwrap();

        assert_official(
            &mut ci_command(&repo, github, &target.to_string()),
            "v1.2.4",
            "v1.2.3",
        );
    }
}

#[test]
fn missing_ci_event_commit_does_not_fall_back_to_head() {
    for github in [false, true] {
        let repo = released_repo();
        repo.repo
            .remote("origin", repo.repo.path().to_str().unwrap())
            .unwrap();
        ci_command(&repo, github, "1111111111111111111111111111111111111111")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(
                "1111111111111111111111111111111111111111",
            ));
    }
}

#[test]
fn github_internal_clone_keeps_full_history_and_checks_out_the_event_commit() {
    let repo = released_repo();
    let candidate = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
    tag(&repo.repo, "v1.2.4-rc.1", candidate, true);
    for index in 0..25 {
        commit(
            &repo.repo,
            "main",
            &format!("chore: change {index}"),
            &[head(&repo.repo)],
        );
    }
    let target = head(&repo.repo);
    let newer = commit(&repo.repo, "main", "feat: newer release", &[target]);
    tag(&repo.repo, "v2.0.0-rc.1", newer, false);
    let server = tempfile::tempdir().unwrap();
    Repository::clone(
        repo.repo.workdir().unwrap().to_str().unwrap(),
        server.path().join("repo.git"),
    )
    .unwrap();
    let clone_path = server.path().join("checkout");
    let mut command = ci_command(&repo, true, &target.to_string());
    command
        .env(
            "GITHUB_SERVER_URL",
            format!("file://{}", server.path().display()),
        )
        .env("GITHUB_REPOSITORY", "repo")
        .env("GITHUB_REF", "refs/heads/main")
        .env("CLONE_TARGET_PATH", &clone_path);
    assert_official(&mut command, "v1.2.4", "v1.2.3");
    let cloned = Repository::open(clone_path).unwrap();
    assert!(!cloned.is_shallow());
    assert_eq!(head(&cloned), target);
}

#[test]
fn different_versions_merged_into_target_are_ambiguous() {
    let repo = released_repo();
    let base = head(&repo.repo);
    let hotfix = commit(&repo.repo, "hotfix/1.2.4", "fix: hotfix", &[base]);
    tag(&repo.repo, "v1.2.4-rc.1", hotfix, false);
    let future = commit(&repo.repo, "release/2.x.x", "feat: future", &[base]);
    tag(&repo.repo, "v2.0.0-rc.1", future, true);
    let merge = commit(&repo.repo, "main", "Merge hotfix", &[base, hotfix]);
    commit(&repo.repo, "main", "Merge future", &[merge, future]);

    repo.command()
        .arg("version")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Ambiguous official release"))
        .stderr(predicate::str::contains("v1.2.4-rc.1"))
        .stderr(predicate::str::contains("v2.0.0-rc.1"));
}

#[test]
fn multiple_prereleases_of_the_same_version_are_not_ambiguous() {
    let repo = released_repo();
    let dev = commit(&repo.repo, "main", "feat: development", &[head(&repo.repo)]);
    tag(&repo.repo, "v1.3.0-dev.1.abcd1234", dev, false);
    let rc1 = commit(&repo.repo, "main", "fix: candidate one", &[dev]);
    tag(&repo.repo, "v1.3.0-rc.1", rc1, false);
    let rc2 = commit(&repo.repo, "main", "fix: candidate two", &[rc1]);
    tag(&repo.repo, "1.3.0-rc.2", rc2, true);
    assert_official(repo.command().arg("version"), "v1.3.0", "v1.2.3");
}

#[test]
fn already_released_candidates_do_not_make_a_new_candidate_ambiguous() {
    let repo = released_repo();
    let base = head(&repo.repo);
    tag(&repo.repo, "v1.1.0-rc.1", base, false);
    tag(&repo.repo, "v1.2.3-rc.1", base, true);
    let candidate = commit(&repo.repo, "main", "fix: hotfix", &[base]);
    tag(&repo.repo, "v1.2.4-rc.1", candidate, false);
    assert_official(repo.command().arg("version"), "v1.2.4", "v1.2.3");
}

#[test]
fn explicit_candidate_resolves_ambiguity_and_overrides_the_environment() {
    let repo = released_repo();
    let hotfix = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
    tag(&repo.repo, "v1.2.4-rc.1", hotfix, true);
    let future = commit(&repo.repo, "main", "feat: future", &[hotfix]);
    tag(&repo.repo, "v2.0.0-rc.1", future, false);

    assert_official(
        repo.command()
            .env("CANDIDATE", "v2.0.0-rc.1")
            .arg("version"),
        "v2.0.0",
        "v1.2.3",
    );
    assert_official(
        repo.command().env("CANDIDATE", "v2.0.0-rc.1").args([
            "version",
            "--candidate",
            "v1.2.4-rc.1",
        ]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn explicit_candidate_can_select_an_original_tag_after_squash_or_rebase() {
    for rebase in [false, true] {
        let repo = released_repo();
        let base = head(&repo.repo);
        let first = commit(&repo.repo, "hotfix/1.2.4", "fix: first change", &[base]);
        let original = commit(&repo.repo, "hotfix/1.2.4", "fix: second change", &[first]);
        tag(&repo.repo, "v1.2.4-rc.1", original, true);
        // Construct the rewritten graph: neither squash nor rebase retains
        // the tagged original commit as a parent of the target.
        let updated_base = commit(&repo.repo, "main", "chore: main advances", &[base]);
        let target = if rebase {
            let rewritten = commit(&repo.repo, "main", "fix: first change", &[updated_base]);
            commit(&repo.repo, "main", "fix: second change", &[rewritten])
        } else {
            commit(
                &repo.repo,
                "main",
                "fix: squash both changes",
                &[updated_base],
            )
        };
        assert!(!repo.repo.graph_descendant_of(target, original).unwrap());
        assert_official(repo.command().arg("version"), "v1.3.0", "v1.2.3");
        assert_official(
            repo.command().args([
                "version",
                "--scope",
                "release",
                "--candidate",
                "v1.2.4-rc.1",
            ]),
            "v1.2.4",
            "v1.2.3",
        );
    }
}

#[test]
fn explicit_candidate_accepts_supported_formats_and_requires_an_exact_name() {
    for name in ["v1.2.4-rc.1", "1.2.4-rc.1", "v1.2.4-dev.2.abcd1234"] {
        let repo = released_repo();
        tag(&repo.repo, name, head(&repo.repo), false);
        assert_official(
            repo.command().args(["version", "--candidate", name]),
            "v1.2.4",
            "v1.2.3",
        );
    }
    let repo = released_repo();
    tag(&repo.repo, "1.2.4-rc.1", head(&repo.repo), false);
    repo.command()
        .args(["version", "--candidate", "v1.2.4-rc.1"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("tag not found"));
}

#[test]
fn explicit_candidate_is_rejected_during_prerelease_generation() {
    for (branch, candidate, official) in [
        ("develop", "v1.2.4-rc.1", "v1.2.4"),
        ("feature/topic", "v1.2.4-rc.1", "v1.2.4"),
        ("release/1.3.x", "v1.3.0-rc.1", "v1.3.0"),
        ("hotfix/1.2.4", "v1.2.4-rc.1", "v1.2.4"),
    ] {
        let repo = TestRepo::new(branch);
        tag(&repo.repo, "v1.2.3", head(&repo.repo), false);
        tag(&repo.repo, candidate, head(&repo.repo), false);
        repo.command()
            .args(["version", "--candidate", candidate])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(
                "--candidate requires official version calculation",
            ));
        assert_official(
            repo.command()
                .args(["version", "--scope", "release", "--candidate", candidate]),
            official,
            "v1.2.3",
        );
    }
}

#[test]
fn invalid_or_missing_candidates_never_fall_back() {
    let repo = released_repo();
    for name in [
        "v1.2.4-rc.invalid",
        "v1.2.4-rc.1.extra",
        "v1.2.4-rc.1-ignored",
        "v1.2.4-alpha.1",
        "v18446744073709551616.2.4-rc.1",
        "v01.2.4-rc.1",
    ] {
        tag(&repo.repo, name, head(&repo.repo), false);
    }
    for name in [
        "v9.0.0-rc.1",
        "main",
        "HEAD",
        "v1.2.3",
        "v1.2.3^{commit}",
        "v1.2.4-rc.invalid",
        "v1.2.4-rc.1.extra",
        "v1.2.4-rc.1-ignored",
        "v1.2.4-alpha.1",
        "v18446744073709551616.2.4-rc.1",
        "v01.2.4-rc.1",
    ] {
        repo.command()
            .args(["version", "--candidate", name])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(format!(
                "Invalid candidate '{name}'"
            )));
    }
    TestRepo::new("main")
        .command()
        .args(["version", "--candidate", "v0.1.0-rc.1"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("tag not found"));
}

#[test]
fn candidate_must_be_newer_than_the_targets_last_official_version() {
    let repo = released_repo();
    for name in ["v1.2.3-rc.1", "v1.1.0-rc.9"] {
        tag(&repo.repo, name, head(&repo.repo), false);
        repo.command()
            .args(["version", "--candidate", name])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("must be newer"));
    }
}

#[test]
fn candidate_cannot_reuse_an_official_version_published_on_another_branch() {
    for published in ["v1.2.4", "1.2.4"] {
        let repo = released_repo();
        let base = head(&repo.repo);
        tag(&repo.repo, "v1.2.4-rc.1", base, false);
        let other = commit(
            &repo.repo,
            "release/1.2.x",
            "fix: published elsewhere",
            &[base],
        );
        tag(&repo.repo, published, other, true);
        repo.command()
            .args(["version", "--candidate", "v1.2.4-rc.1"])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(format!(
                "already exists as tag '{published}'"
            )));
    }
}

#[test]
fn candidate_must_point_to_a_commit() {
    let repo = released_repo();
    let blob = repo.repo.blob(b"not a commit").unwrap();
    let object = repo.repo.find_object(blob, None).unwrap();
    repo.repo
        .tag_lightweight("v1.2.4-rc.1", &object, false)
        .unwrap();
    repo.command()
        .args(["version", "--candidate", "v1.2.4-rc.1"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("tag must point to a commit"));
}

#[test]
fn stable_bumps_and_explicit_candidates_ignore_unneeded_prerelease_objects() {
    let repo = released_repo();
    tag(&repo.repo, "v1.2.4-rc.1", head(&repo.repo), false);
    let blob = repo.repo.blob(b"not a release commit").unwrap();
    let object = repo.repo.find_object(blob, None).unwrap();
    repo.repo
        .tag_lightweight("v9.0.0-rc.1", &object, false)
        .unwrap();

    assert_official(
        repo.command().args(["version", "--scope", "patch"]),
        "v1.2.4",
        "v1.2.3",
    );
    assert_official(
        repo.command()
            .args(["version", "--candidate", "v1.2.4-rc.1"]),
        "v1.2.4",
        "v1.2.3",
    );
}

#[test]
fn explicit_candidate_still_requires_complete_history_for_last_version() {
    let repo = released_repo();
    let target = commit(&repo.repo, "main", "fix: hotfix", &[head(&repo.repo)]);
    tag(&repo.repo, "v1.2.4-rc.1", target, false);
    std::fs::write(repo.repo.path().join("shallow"), format!("{target}\n")).unwrap();
    repo.command()
        .args(["version", "--candidate", "v1.2.4-rc.1"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("requires complete history"));
}

#[test]
fn prerelease_base_ignores_unmerged_official_tags_and_advances_after_merge() {
    for annotated in [false, true] {
        for (branch, core, stage) in [
            ("develop", "1.3.0", "dev"),
            ("feature/topic", "1.3.0", "dev"),
            ("release/1.3.x", "1.3.0", "rc"),
            ("hotfix/1.2.4", "1.2.4", "rc"),
        ] {
            let repo = released_repo();
            let base = head(&repo.repo);
            let target = commit(&repo.repo, branch, "current work", &[base]);
            let future = commit(&repo.repo, "future", "separate release", &[base]);
            tag(&repo.repo, "2.0.0", future, annotated);
            repo.repo.set_head(&format!("refs/heads/{branch}")).unwrap();

            let suffix = if stage == "dev" {
                format!(".{}", &target.to_string()[..8])
            } else {
                String::new()
            };
            assert_official(
                repo.command().arg("version"),
                &format!("v{core}-{stage}.1{suffix}"),
                "v1.2.3",
            );

            if stage == "dev" {
                let merged = commit(&repo.repo, branch, "merge release", &[target, future]);
                assert_official(
                    repo.command().arg("version"),
                    &format!("v2.1.0-dev.1.{}", &merged.to_string()[..8]),
                    "v2.0.0",
                );
            }
        }
    }
}

#[test]
fn prerelease_counters_and_last_version_follow_target_history() {
    let repo = released_repo();
    let base = head(&repo.repo);
    let target = commit(&repo.repo, "develop", "development", &[base]);
    let future = commit(&repo.repo, "future", "separate release", &[base]);
    for name in ["v2.0.0", "v2.1.0-dev.99.abcdef12", "v2.1.0-rc.99"] {
        tag(&repo.repo, name, future, false);
    }
    let candidate = commit(&repo.repo, "candidate", "unmerged prereleases", &[base]);
    tag(&repo.repo, "v1.3.0-dev.10.abcdef12", candidate, true);
    tag(&repo.repo, "v1.3.0-rc.7", candidate, true);
    repo.repo.set_head("refs/heads/develop").unwrap();
    assert_official(
        repo.command().arg("version"),
        &format!("v1.3.0-dev.1.{}", &target.to_string()[..8]),
        "v1.2.3",
    );
    assert_official(
        repo.command().args(["version", "--stage", "rc"]),
        "v1.3.0-rc.1",
        "v1.2.3",
    );

    tag(&repo.repo, "v1.3.0-dev.2.abcdef12", target, false);
    tag(&repo.repo, "v1.3.0-rc.3", target, false);
    assert_official(
        repo.command().arg("version"),
        &format!("v1.3.0-dev.3.{}", &target.to_string()[..8]),
        "v1.3.0-dev.2.abcdef12",
    );
    assert_official(
        repo.command().args(["version", "--stage", "rc"]),
        "v1.3.0-rc.4",
        "v1.3.0-rc.3",
    );

    let merged = commit(
        &repo.repo,
        "develop",
        "merge prereleases",
        &[target, candidate],
    );
    assert_official(
        repo.command().arg("version"),
        &format!("v1.3.0-dev.11.{}", &merged.to_string()[..8]),
        "v1.3.0-dev.10.abcdef12",
    );
    assert_official(
        repo.command().args(["version", "--stage", "rc"]),
        "v1.3.0-rc.8",
        "v1.3.0-rc.7",
    );
}

#[test]
fn prerelease_base_defaults_to_zero_when_no_official_tag_is_reachable() {
    let repo = TestRepo::new("develop");
    let target = head(&repo.repo);
    let future = commit(&repo.repo, "future", "separate release", &[target]);
    tag(&repo.repo, "v2.0.0", future, false);
    tag(&repo.repo, "v0.1.0-dev.99.abcdef12", future, false);
    tag(&repo.repo, "v0.1.0-rc.99", future, false);
    assert_official(
        repo.command().arg("version"),
        &format!("v0.1.0-dev.1.{}", &target.to_string()[..8]),
        "v0.0.0",
    );
    assert_official(
        repo.command().args(["version", "--stage", "rc"]),
        "v0.1.0-rc.1",
        "v0.0.0",
    );
}

#[test]
fn prerelease_base_requires_complete_history_even_when_tag_objects_exist() {
    let repo = released_repo();
    let target = commit(&repo.repo, "develop", "development", &[head(&repo.repo)]);
    repo.repo.set_head("refs/heads/develop").unwrap();
    std::fs::write(repo.repo.path().join("shallow"), format!("{target}\n")).unwrap();
    for stage in ["dev", "rc"] {
        repo.command()
            .args(["version", "--stage", stage])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains("requires complete history"));
    }
}

#[test]
fn ci_prerelease_versions_use_the_event_history_instead_of_a_newer_checkout() {
    for github in [false, true] {
        let repo = released_repo();
        let target = commit(&repo.repo, "main", "development event", &[head(&repo.repo)]);
        tag(&repo.repo, "v1.3.0-dev.2.abcdef12", target, false);
        tag(&repo.repo, "v1.3.0-rc.3", target, false);
        let newer = commit(&repo.repo, "main", "newer release", &[target]);
        tag(&repo.repo, "v2.0.0", newer, true);
        tag(&repo.repo, "v1.3.0-dev.10.abcdef12", newer, true);
        tag(&repo.repo, "v1.3.0-rc.7", newer, true);
        repo.repo
            .remote("origin", repo.repo.path().to_str().unwrap())
            .unwrap();
        let branch_variable = if github {
            "GITHUB_REF_NAME"
        } else {
            "CI_COMMIT_REF_NAME"
        };

        assert_official(
            ci_command(&repo, github, &target.to_string()).env(branch_variable, "develop"),
            &format!("v1.3.0-dev.3.{}", &target.to_string()[..8]),
            "v1.3.0-dev.2.abcdef12",
        );
        assert_official(
            ci_command(&repo, github, &target.to_string())
                .env(branch_variable, "develop")
                .args(["--stage", "rc"]),
            "v1.3.0-rc.4",
            "v1.3.0-rc.3",
        );
        for stage in ["dev", "rc"] {
            ci_command(&repo, github, "1111111111111111111111111111111111111111")
                .env(branch_variable, "develop")
                .args(["--stage", stage])
                .assert()
                .failure()
                .stdout("")
                .stderr(predicate::str::contains(
                    "1111111111111111111111111111111111111111",
                ));
        }
    }
}
