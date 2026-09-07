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
    for branch in ["develop", "feature/topic", "release/1.2.x", "hotfix/1.2.4"] {
        let repo = TestRepo::new(branch);
        tag(&repo.repo, "v1.2.3", head(&repo.repo), false);
        tag(&repo.repo, "v1.2.4-rc.1", head(&repo.repo), false);
        repo.command()
            .args(["version", "--candidate", "v1.2.4-rc.1"])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(
                "--candidate requires official version calculation",
            ));
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
