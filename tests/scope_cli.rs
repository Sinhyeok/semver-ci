use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn init_git_repo(dir: &Path, branch: &str) {
    // init
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());

    // config
    let status = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.name", "Test User"])
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.email", "test@example.com"])
        .status()
        .unwrap();
    assert!(status.success());

    // commit
    std::fs::write(dir.join("README.md"), "temp repo").unwrap();
    let status = Command::new("git")
        .current_dir(dir)
        .args(["add", "."])
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("git")
        .current_dir(dir)
        .args(["commit", "-q", "-m", "chore: init"])
        .status()
        .unwrap();
    assert!(status.success());

    // branch (create or switch)
    let status = Command::new("git")
        .current_dir(dir)
        .args(["switch", "-C", branch])
        .status()
        .unwrap();
    assert!(status.success());
}

fn run_scope(dir: &Path) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("svci").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("ENVIRONMENT", "test");
    cmd.env("GITHUB_ACTIONS", "false");
    cmd.env("GITLAB_CI", "false");
    cmd.env("CLONE_TARGET_PATH", dir.to_str().unwrap());
    cmd.arg("scope");
    cmd.assert()
}

#[test]
fn scope_outputs_minor_on_develop() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "develop");
    run_scope(tmp.path())
        .success()
        .stdout(predicate::str::is_match("^(minor)\\n$").unwrap());
}

#[test]
fn scope_outputs_major_on_release_branch() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "release/1.x.x");
    run_scope(tmp.path())
        .success()
        .stdout(predicate::str::is_match("^(major)\\n$").unwrap());
}

#[test]
fn scope_outputs_patch_on_hotfix_branch() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "hotfix/1.2.3");
    run_scope(tmp.path())
        .success()
        .stdout(predicate::str::is_match("^(patch)\\n$").unwrap());
}

#[test]
fn scope_outputs_release_on_main() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "main");
    run_scope(tmp.path())
        .success()
        .stdout(predicate::str::is_match("^(release)\\n$").unwrap());
}
