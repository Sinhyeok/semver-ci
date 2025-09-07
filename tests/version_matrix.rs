use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn sh(args: &[&str]) {
    let status = Command::new("git").args(args).status().unwrap();
    assert!(status.success());
}

fn sh_in(dir: &Path, args: &[&str]) {
    let status = Command::new("git").current_dir(dir).args(args).status().unwrap();
    assert!(status.success());
}

fn init_git_repo(dir: &Path, branch: &str) {
    // init
    sh(&["init", "-q", dir.to_str().unwrap()]);

    // configure user
    sh_in(dir, &["config", "user.name", "Test User"]);
    sh_in(dir, &["config", "user.email", "test@example.com"]);

    // initial commit
    fs::write(dir.join("README.md"), "temp repo").unwrap();
    sh_in(dir, &["add", "."]);
    sh_in(dir, &["commit", "-q", "-m", "chore: init"]);

    // create or switch branch
    sh_in(dir, &["switch", "-C", branch]);
}

fn tag(dir: &Path, name: &str) {
    sh_in(dir, &["tag", name]);
}

fn run_svci_in(dir: &Path) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("svci").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("ENVIRONMENT", "test");
    cmd.env("GITHUB_ACTIONS", "false");
    cmd.env("GITLAB_CI", "false");
    cmd.env("GIT_TOKEN", "test-token");
    cmd.env("CLONE_TARGET_PATH", dir.to_str().unwrap());
    cmd.env("FORCE_FETCH_TAGS", "false");
    cmd.arg("version");
    cmd.assert()
}

#[test]
fn develop_with_no_tags_defaults_from_0_0_0() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "develop");

    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v0.1.0-dev.1"))
        .stdout(predicate::str::contains("LAST_VERSION=v0.0.0"));
}

#[test]
fn feature_with_official_tag_only() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "feature/xyz");
    tag(tmp.path(), "v1.2.3");

    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v1.3.0-dev.1"))
        .stdout(predicate::str::contains("LAST_VERSION=v1.2.3"));
}

#[test]
fn feature_with_prerelease_tag_advances_prerelease() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "feature/abc");
    tag(tmp.path(), "v1.2.3");
    tag(tmp.path(), "v1.3.0-dev.1.abcd1234");

    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v1.3.0-dev.2"))
        .stdout(predicate::str::contains("LAST_VERSION=v1.3.0-dev.1.abcd1234"));
}

#[test]
fn release_branch_with_prerelease_yields_official_release() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "release/1.3.x");
    tag(tmp.path(), "v1.2.3");
    tag(tmp.path(), "v1.3.0-rc.1");

    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v1.3.0-rc.2"))
        .stdout(predicate::str::contains("LAST_VERSION=v1.3.0-rc.1"));
}

#[test]
fn main_branch_treats_as_release_scope() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "main");
    tag(tmp.path(), "v1.2.3");
    tag(tmp.path(), "v1.3.0-rc.2");

    // 기본 동작은 prerelease_stage 공백이므로 upcoming_official_version 로직이 동작
    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v1.3.0"))
        .stdout(predicate::str::contains("LAST_VERSION=v1.2.3"));
}

#[test]
fn main_branch_with_prerelease_older_than_official_bumps_minor() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "main");
    // 최신 official tag와 동일 버전의 오래된 prerelease tag 존재
    tag(tmp.path(), "v1.2.3");
    tag(tmp.path(), "v1.2.3-rc.2");

    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v1.3.0"))
        .stdout(predicate::str::contains("LAST_VERSION=v1.2.3"));
}

#[test]
fn release_branch_with_no_tags() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "release/0.1.x");

    // 태그가 전혀 없는 상태
    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v0.1.0-rc.1"))
        .stdout(predicate::str::contains("LAST_VERSION=v0.0.0"));
}

#[test]
fn main_branch_with_no_tags() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path(), "main");

    // 태그가 전혀 없는 상태
    run_svci_in(tmp.path())
        .success()
        .stdout(predicate::str::contains("UPCOMING_VERSION=v0.1.0"))
        .stdout(predicate::str::contains("LAST_VERSION=v0.0.0"));
}


