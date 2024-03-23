use crate::semantic_version::SemanticVersion;
use git2::string_array::StringArray;
use git2::{
    Config, Cred, CredentialType, Error, FetchOptions, ObjectType, Oid, PushOptions,
    RemoteCallbacks, Repository,
};
use regex::Regex;
use std::env;
use std::ops::Not;
use std::path::Path;

pub(crate) fn tag_names(
    repo_path: &str,
    force_fetch_tags: bool,
    git_username: &str,
    git_token: &str,
) -> Result<StringArray, Error> {
    let repo =
        Repository::open(repo_path)?;

    if force_fetch_tags {
        fetch_refs(&repo, git_username, git_token, &["refs/tags/*:refs/tags/*"])?;
    }

    repo.tag_names(None)
}

pub(crate) fn last_tag_by_pattern(
    tag_names: StringArray,
    tag_pattern: &str,
    default: &str,
) -> String {
    let tag_regex = Regex::new(tag_pattern).unwrap();
    let mut valid_versions: Vec<SemanticVersion> = vec![];

    for tag_name in tag_names.iter().flatten() {
        if tag_regex.is_match(tag_name).not() {
            continue;
        }

        match SemanticVersion::from_string(tag_name.to_string()) {
            Ok(version) => valid_versions.push(version),
            Err(msg) => eprintln!("{}", msg),
        }
    }

    valid_versions.sort_by(|a, b| b.cmp(a));

    match valid_versions.first() {
        Some(version) => version.to_string(true),
        None => default.to_string(),
    }
}

pub(crate) fn branch_name(repo_path: &str) -> Result<String, Error> {
    let repo = Repository::open(repo_path)?;

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

pub(crate) fn short_commit_sha(repo_path: &str) -> Result<String, Error> {
    let repo = Repository::open(repo_path)?;

    let commit_sha = repo.head()?.peel_to_commit()?.id().to_string();

    Ok(commit_sha[..8].to_string())
}

pub(crate) fn get_config_value(repo_path: &str, name: &str) -> Option<String> {
    let repo = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => return None,
    };

    let config = match repo.config() {
        Ok(config) => config,
        Err(_) => return None,
    };

    let value = match config.get_entry(name) {
        Ok(entry) => entry.value().map(|username| username.to_string()),
        Err(_) => None,
    };

    value
}

pub(crate) fn set_config_value(repo_path: &str, name: &str, value: &str) -> Result<(), Error> {
    let repo = Repository::open(repo_path)?;

    let mut config = repo.config()?;
    config.set_str(name, value)
}

pub(crate) fn set_global_config_value(name: &str, value: &str) -> Result<(), Error> {
    let mut config = Config::open_default()?;
    config.set_str(name, value)
}

pub(crate) fn clone(
    url: &str,
    target_path: &str,
    user: &str,
    token: &str,
    depth: i32,
) -> Result<Repository, Error> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    fetch_options.remote_callbacks(callbacks);
    fetch_options.depth(depth);

    git2::build::RepoBuilder::new()
        .fetch_options(fetch_options)
        .clone(url, Path::new(target_path))
}

pub(crate) fn checkout(repo: &Repository, ref_name: &str) -> Result<(), Error> {
    let reference = repo.revparse_single(ref_name)?;
    repo.checkout_tree(&reference, None)?;
    repo.set_head_detached(reference.id())
}

pub(crate) fn fetch_refs(
    repo: &Repository,
    user: &str,
    token: &str,
    refspecs: &[&str],
) -> Result<(), Error> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    fetch_options.remote_callbacks(callbacks);

    repo.find_remote("origin")?
        .fetch(refspecs, Some(&mut fetch_options), None)
}

pub(crate) fn tag(
    repo: &Repository,
    tag_name: &str,
    tag_message: &str,
    user: &str,
    email: &str,
) -> Result<Oid, Error> {
    let head = repo.head()?;
    let git_object = head.peel(ObjectType::Any)?;
    let tagger = git2::Signature::now(user, email)?;

    repo.tag(tag_name, &git_object, &tagger, tag_message, false)
}

pub(crate) fn push_tag(
    repo: &Repository,
    user: &str,
    token: &str,
    tag_name: &str,
) -> Result<(), Error> {
    let mut push_options = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    push_options.remote_callbacks(callbacks);

    let ref_spec = format!("refs/tags/{}", tag_name);
    repo.find_remote("origin")?
        .push(&[ref_spec], Some(&mut push_options))
}

fn git_auth_callback(
    cred: CredentialType,
    username: Option<&str>,
    user: &str,
    token: &str,
) -> Result<Cred, Error> {
    if cred.is_ssh_key() {
        let ssh_username = username.unwrap_or(user);
        Cred::ssh_key(
            ssh_username,
            None,
            Path::new(&ssh_key_path()),
            ssh_key_passphrase().as_deref(),
        )
    } else if cred.is_user_pass_plaintext() {
        let plain_username = username.unwrap_or(user);
        Cred::userpass_plaintext(plain_username, token)
    } else {
        panic!("Unexpected CredentialType: {:?}", cred)
    }
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
