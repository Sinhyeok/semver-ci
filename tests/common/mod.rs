use assert_cmd::prelude::*;
use git2::{Repository, RepositoryInitOptions, Signature};
use std::process::Command;
use tempfile::TempDir;

pub struct TestRepo {
    pub repo: Repository,
    dir: TempDir,
}

impl TestRepo {
    pub fn new(branch: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head(branch);
        let repo = Repository::init_opts(dir.path(), &options).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Commit through libgit2 so user signing settings and hooks cannot run.
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "chore: init",
                &tree,
                &[],
            )
            .unwrap();
        }

        // Stop dotenv from searching the temporary directory's parents.
        std::fs::write(dir.path().join(".env"), "").unwrap();
        Self { repo, dir }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::cargo_bin("svci").expect("binary exists");
        command
            .current_dir(self.dir.path())
            // Set variables on the child only; tests can run in parallel.
            .env_clear()
            .env("ENVIRONMENT", "test")
            .env("GITHUB_ACTIONS", "false")
            .env("GITLAB_CI", "false")
            .env("GIT_TOKEN", "test-token")
            .env("CLONE_TARGET_PATH", self.dir.path())
            .env("FORCE_FETCH_TAGS", "false")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("empty.gitconfig"));
        command
    }
}
