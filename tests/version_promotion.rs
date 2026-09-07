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
