mod common;

use assert_cmd::prelude::*;
use common::TestRepo;
use git2::Repository;
use predicates::prelude::*;
use tempfile::TempDir;

fn add_origin(repo: &TestRepo, dir: &TempDir) -> Repository {
    let origin = Repository::init_bare(dir.path()).unwrap();
    repo.repo
        .remote("origin", dir.path().to_str().unwrap())
        .unwrap();
    origin
}

fn assert_pushed_tag(repo: &TestRepo, origin: &Repository, name: &str, message: &str) {
    let reference = format!("refs/tags/{name}");
    let local_tag = repo.repo.find_reference(&reference).unwrap();
    let remote_tag = origin.find_reference(&reference).unwrap();
    assert_eq!(local_tag.target(), remote_tag.target());

    let tag = origin.find_tag(remote_tag.target().unwrap()).unwrap();
    assert_eq!(tag.name(), Some(name));
    assert_eq!(tag.message(), Some(message));
    assert_eq!(tag.target_id(), repo.repo.head().unwrap().target().unwrap());
    let tagger = tag.tagger().unwrap();
    assert_eq!(tagger.name(), Some("Test User"));
    assert_eq!(tagger.email(), Some("test@example.com"));
}

#[test]
fn tag_creates_and_pushes_an_annotated_tag_at_head() {
    let repo = TestRepo::new("main");
    let origin_dir = TempDir::new().unwrap();
    let origin = add_origin(&repo, &origin_dir);
    repo.command()
        .args(["tag", "v1.2.3", "--tag-message", "Release 1.2.3"])
        .assert()
        .success();
    assert_pushed_tag(&repo, &origin, "v1.2.3", "Release 1.2.3");
}

#[test]
fn tag_strips_one_leading_v_and_accepts_unprefixed_names() {
    for (input, expected) in [
        ("v1.2.3", "1.2.3"),
        ("1.2.3", "1.2.3"),
        ("vv1.2.3", "v1.2.3"),
    ] {
        let repo = TestRepo::new("main");
        let origin_dir = TempDir::new().unwrap();
        let origin = add_origin(&repo, &origin_dir);
        repo.command()
            .args(["tag", input, "--strip-prefix-v"])
            .assert()
            .success();
        assert_pushed_tag(&repo, &origin, expected, "");
        assert_eq!(repo.repo.tag_names(None).unwrap().len(), 1);
        assert_eq!(origin.tag_names(None).unwrap().len(), 1);
    }
}

#[test]
fn tag_reads_options_from_environment_and_prefers_explicit_message() {
    let repo = TestRepo::new("main");
    let origin_dir = TempDir::new().unwrap();
    let origin = add_origin(&repo, &origin_dir);
    repo.command()
        .env("STRIP_PREFIX_V", "true")
        .env("TAG_MESSAGE", "Environment message")
        .args(["tag", "v1.2.3"])
        .assert()
        .success();
    assert_pushed_tag(&repo, &origin, "1.2.3", "Environment message");
    repo.command()
        .env("TAG_MESSAGE", "Environment message")
        .args(["tag", "v1.2.4", "--tag-message", "Explicit message"])
        .assert()
        .success();
    assert_pushed_tag(&repo, &origin, "v1.2.4", "Explicit message");
}

#[test]
fn duplicate_tag_fails_without_overwriting_the_local_or_remote_tag() {
    let repo = TestRepo::new("main");
    let origin_dir = TempDir::new().unwrap();
    let origin = add_origin(&repo, &origin_dir);
    repo.command()
        .args(["tag", "v1.2.3", "--tag-message", "Original"])
        .assert()
        .success();
    let original = repo.repo.refname_to_id("refs/tags/v1.2.3").unwrap();
    repo.command()
        .args(["tag", "v1.2.3", "--tag-message", "Replacement"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
    assert_pushed_tag(&repo, &origin, "v1.2.3", "Original");
    assert_eq!(
        repo.repo.refname_to_id("refs/tags/v1.2.3").unwrap(),
        original
    );
}

#[test]
fn tag_reports_push_failure_when_origin_is_missing() {
    TestRepo::new("main")
        .command()
        .args(["tag", "v1.2.3"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to push tag"))
        .stderr(predicate::str::contains("origin"));
}
