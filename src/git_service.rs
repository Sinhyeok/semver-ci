use crate::pipelines;
use git2::{Error, FetchOptions, RemoteCallbacks, Repository};
use regex::Regex;
use std::env;
use std::path::Path;

const SEMANTIC_VERSION_TAG_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";

pub(crate) fn last_semantic_version_tag(default: String) -> String {
    let repo = Repository::open(".").unwrap_or_else(|e| panic!("Failed to open git repo: {}", e));

    let semantic_version_regex = Regex::new(SEMANTIC_VERSION_TAG_PATTERN).unwrap();

    let pipeline_info = pipelines::pipeline_info();
    if pipeline_info.force_fetch_tags {
        fetch_tags(&repo, &pipeline_info.git_username, &pipeline_info.git_token)
            .unwrap_or_else(|e| panic!("Failed to retrieve tags: {}", e));
    }

    let tag_names = repo
        .tag_names(None)
        .unwrap_or_else(|e| panic!("Failed to retrieve tags: {}", e));

    tag_names
        .iter()
        .flatten()
        .filter(|t| semantic_version_regex.is_match(t))
        .last()
        .map_or(default, |tag_name| tag_name.to_string())
}

pub(crate) fn branch_name() -> Result<String, Error> {
    let repo = Repository::open(".")?;

    let head = repo.head()?;

    if head.is_branch() {
        if let Some(branch) = head.shorthand() {
            Ok(branch.to_string())
        } else {
            Err(Error::from_str("Failed to retrieve branch name"))
        }
    } else {
        Err(Error::from_str(
            "HEAD is in detached state, not pointing to branch",
        ))
    }
}

pub(crate) fn short_commit_sha() -> Result<String, Error> {
    let repo = Repository::open(".")?;

    let commit_sha = repo.head()?.peel_to_commit()?.id().to_string();

    Ok(commit_sha[..8].to_string())
}

fn fetch_tags(repo: &Repository, user: &str, token: &str) -> Result<(), Error> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username, cred| {
        if cred.is_ssh_key() {
            let ssh_username = username.unwrap_or(user);
            git2::Cred::ssh_key(
                ssh_username,
                None,
                Path::new(&ssh_key_path()),
                ssh_key_passphrase().as_deref(),
            )
        } else if cred.is_user_pass_plaintext() {
            let plain_username = username.unwrap_or(user);
            git2::Cred::userpass_plaintext(plain_username, token)
        } else {
            panic!("Unexpected CredentialType: {:?}", cred)
        }
    });

    fetch_options.remote_callbacks(callbacks);

    repo.find_remote("origin")?.fetch(
        &["refs/tags/*:refs/tags/*"],
        Some(&mut fetch_options),
        None,
    )?;

    Ok(())
}

fn ssh_key_path() -> String {
    env::var("GIT_SSH_KEY_PATH").unwrap_or_else(|e| panic!("{}: \"GIT_SSH_KEY_PATH\"", e))
}

fn ssh_key_passphrase() -> Option<String> {
    match env::var("GIT_SSH_KEY_PASSPHRASE") {
        Ok(s) => Some(s),
        Err(_e) => None,
    }
}
