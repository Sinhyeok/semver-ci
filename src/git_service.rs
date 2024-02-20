use crate::pipelines::PipelineInfo;
use git2::{
    Config, Cred, CredentialType, Error, FetchOptions, ObjectType, Oid, PushOptions,
    RemoteCallbacks, Repository,
};
use regex::Regex;
use std::env;
use std::path::Path;

const SEMANTIC_VERSION_TAG_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";

pub(crate) fn last_semantic_version_tag(default: String, pipeline_info: &PipelineInfo) -> String {
    let repo = Repository::open(".").unwrap_or_else(|e| panic!("Failed to open git repo: {}", e));

    let semantic_version_regex = Regex::new(SEMANTIC_VERSION_TAG_PATTERN).unwrap();

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

pub(crate) fn tag_and_push(
    pipeline_info: &PipelineInfo,
    tag_name: &str,
    tag_message: &str,
) -> Result<(), Error> {
    let repo = Repository::open(".")?;

    tag(
        &repo,
        tag_name,
        tag_message,
        &pipeline_info.git_username,
        &pipeline_info.git_email,
    )?;
    push_tag(
        &repo,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
        tag_name,
    )
}

pub(crate) fn get_config_value(name: &str) -> Option<String> {
    let repo = match Repository::open(".") {
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

pub(crate) fn set_config_value(name: &str, value: &str) -> Result<(), Error> {
    let repo = Repository::open(".")?;

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

fn fetch_tags(repo: &Repository, user: &str, token: &str) -> Result<(), Error> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    fetch_options.remote_callbacks(callbacks);

    repo.find_remote("origin")?
        .fetch(&["refs/tags/*:refs/tags/*"], Some(&mut fetch_options), None)
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

fn tag(
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

fn push_tag(repo: &Repository, user: &str, token: &str, tag_name: &str) -> Result<(), Error> {
    let mut push_options = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    push_options.remote_callbacks(callbacks);

    let ref_spec = format!("refs/tags/{}", tag_name);
    repo.find_remote("origin")?
        .push(&[ref_spec], Some(&mut push_options))
}
