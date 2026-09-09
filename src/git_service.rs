use crate::config;
use crate::errors::{messages, DefaultError, Result, ResultExt};
use git2::{
    Config, Cred, CredentialType, FetchOptions, ObjectType, Oid, PushOptions, RemoteCallbacks,
    Repository,
};
use std::path::Path;

pub(crate) fn open_repository(path: &str) -> Result<Repository> {
    Repository::open(path).context(messages::open_repository(path))
}

pub(crate) fn tag_names(
    repo_path: &str,
    force_fetch_tags: bool,
    git_username: &str,
    git_token: &str,
) -> Result<Vec<String>> {
    let repo = open_repository(repo_path)?;

    if force_fetch_tags {
        fetch_refs(&repo, git_username, git_token, &["refs/tags/*:refs/tags/*"])?;
    }

    Ok(repo
        .tag_names(None)
        .context(messages::RETRIEVE_TAGS)?
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect())
}

pub(crate) fn reachable_tag_names(
    repo_path: &str,
    tag_names: &[String],
    target: &str,
) -> Result<Vec<String>> {
    let repo = open_repository(repo_path)?;
    if repo.is_shallow() {
        return Err(DefaultError::new(messages::COMPLETE_HISTORY));
    }
    let target = repo
        .revparse_single(target)
        .and_then(|object| object.peel_to_commit())
        .context(messages::target_commit(target))?
        .id();
    let mut reachable = Vec::new();
    for tag_name in tag_names {
        let commit = repo
            .find_reference(&format!("refs/tags/{tag_name}"))
            .and_then(|reference| reference.peel_to_commit())
            .context(messages::tag_commit(tag_name))?
            .id();
        if target == commit
            || repo
                .graph_descendant_of(target, commit)
                .context(messages::check_ancestry(tag_name))?
        {
            reachable.push(tag_name.clone());
        }
    }
    Ok(reachable)
}

pub(crate) fn tag_commit_id(repo_path: &str, tag_name: &str) -> Result<Oid> {
    let repo = open_repository(repo_path)?;
    let commit = repo
        .find_reference(&format!("refs/tags/{tag_name}"))
        .and_then(|reference| reference.peel_to_commit())
        .context(messages::tag_commit(tag_name))?;
    Ok(commit.id())
}

pub(crate) fn branch_name(repo_path: &str) -> Result<String> {
    let repo = open_repository(repo_path)?;

    let head = repo.head().context(messages::RETRIEVE_BRANCH)?;

    if head.is_branch() {
        if let Some(branch) = head.shorthand() {
            Ok(branch.to_string())
        } else {
            Err(DefaultError::new(messages::RETRIEVE_BRANCH))
        }
    } else {
        Err(DefaultError::new(messages::DETACHED_HEAD))
    }
}

pub(crate) fn short_commit_sha(repo_path: &str) -> Result<String> {
    let repo = open_repository(repo_path)?;

    let commit_sha = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .context(messages::RETRIEVE_SHA)?
        .id()
        .to_string();

    Ok(commit_sha[..8].to_string())
}

pub(crate) fn get_config_value(repo_path: &str, name: &str) -> Result<Option<String>> {
    let repo = open_repository(repo_path)?;
    let config = repo.config().context(messages::READ_GIT_CONFIG)?;
    match config.get_string(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(DefaultError::new(messages::read_git_config(name)).with_source(error)),
    }
}

pub(crate) fn set_config_value(repo_path: &str, name: &str, value: &str) -> Result<()> {
    let repo = open_repository(repo_path)?;

    let mut config = repo.config().context(messages::READ_GIT_CONFIG)?;
    config
        .set_str(name, value)
        .context(messages::set_git_config(name))
}

pub(crate) fn set_global_config_value(name: &str, value: &str) -> Result<()> {
    let mut config = Config::open_default().context(messages::READ_GIT_CONFIG)?;
    config
        .set_str(name, value)
        .context(messages::set_git_config(name))
}

pub(crate) fn clone(
    url: &str,
    target_path: &str,
    user: &str,
    token: &str,
    depth: i32,
) -> Result<Repository> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    fetch_options.remote_callbacks(callbacks);
    fetch_options.depth(depth);

    git2::build::RepoBuilder::new()
        .fetch_options(fetch_options)
        .clone(url, Path::new(target_path))
        .context(messages::clone_repository(target_path))
}

pub(crate) fn checkout(repo: &Repository, ref_name: &str) -> Result<()> {
    let reference = repo
        .revparse_single(ref_name)
        .context(messages::checkout(ref_name))?;
    repo.checkout_tree(&reference, None)
        .context(messages::checkout(ref_name))?;
    repo.set_head_detached(reference.id())
        .context(messages::checkout(ref_name))
}

pub(crate) fn fetch_refs(
    repo: &Repository,
    user: &str,
    token: &str,
    refspecs: &[&str],
) -> Result<()> {
    fetch_refs_with_depth(repo, user, token, refspecs, 0)
}

pub(crate) fn fetch_complete_history(
    repo_path: &str,
    target_commit: &str,
    user: &str,
    token: &str,
) -> Result<()> {
    let repo = open_repository(repo_path)?;
    if !repo.is_shallow() {
        return Ok(());
    }

    eprintln!("Shallow CI checkout detected; fetching complete history from origin");
    // libgit2's GIT_FETCH_DEPTH_UNSHALLOW; depth 0 leaves existing shallow roots intact.
    fetch_refs_with_depth(
        &repo,
        user,
        token,
        &[
            "+refs/heads/*:refs/remotes/origin/*",
            "refs/tags/*:refs/tags/*",
            target_commit,
        ],
        i32::MAX,
    )
    .context(messages::FETCH_COMPLETE_HISTORY)?;

    if repo.is_shallow() {
        return Err(DefaultError::new(messages::INCOMPLETE_FETCH));
    }
    Ok(())
}

fn fetch_refs_with_depth(
    repo: &Repository,
    user: &str,
    token: &str,
    refspecs: &[&str],
    depth: i32,
) -> Result<()> {
    let mut fetch_options = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    fetch_options.remote_callbacks(callbacks);
    fetch_options.depth(depth);

    repo.find_remote("origin")
        .and_then(|mut remote| remote.fetch(refspecs, Some(&mut fetch_options), None))
        .context(messages::fetch_refs(refspecs))
}

pub(crate) fn tag(
    repo: &Repository,
    tag_name: &str,
    tag_message: &str,
    user: &str,
    email: &str,
) -> Result<Oid> {
    let head = repo.head().context(messages::create_tag(tag_name))?;
    let git_object = head
        .peel(ObjectType::Any)
        .context(messages::create_tag(tag_name))?;
    let tagger = git2::Signature::now(user, email).context(messages::create_tag(tag_name))?;

    repo.tag(tag_name, &git_object, &tagger, tag_message, false)
        .context(messages::create_tag(tag_name))
}

pub(crate) fn push_tag(repo: &Repository, user: &str, token: &str, tag_name: &str) -> Result<()> {
    let mut push_options = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, cred| git_auth_callback(cred, username, user, token));

    push_options.remote_callbacks(callbacks);

    let ref_spec = format!("refs/tags/{}", tag_name);
    repo.find_remote("origin")
        .and_then(|mut remote| remote.push(&[ref_spec], Some(&mut push_options)))
        .context(messages::push_tag(tag_name))
}

fn git_auth_callback(
    cred: CredentialType,
    username: Option<&str>,
    user: &str,
    token: &str,
) -> std::result::Result<Cred, git2::Error> {
    if cred.is_ssh_key() {
        let ssh_username = username.unwrap_or(user);
        Cred::ssh_key(
            ssh_username,
            None,
            Path::new(&config::env_var("GIT_SSH_KEY_PATH").map_err(auth_error)?),
            config::optional_env_var("GIT_SSH_KEY_PASSPHRASE")
                .map_err(auth_error)?
                .as_deref(),
        )
    } else if cred.is_user_pass_plaintext() {
        let plain_username = username.unwrap_or(user);
        Cred::userpass_plaintext(plain_username, token)
    } else {
        Err(git2::Error::from_str(&messages::unsupported_credentials(
            cred,
        )))
    }
}

// libgit2 requires its own error type at this callback boundary.
fn auth_error(error: DefaultError) -> git2::Error {
    git2::Error::from_str(&error.report())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_credentials_return_an_error_across_the_libgit2_boundary() {
        for credential in [CredentialType::DEFAULT, CredentialType::USERNAME] {
            let error = git_auth_callback(credential, None, "test-user", "private-token")
                .err()
                .expect("unsupported credentials must fail");
            assert!(error.message().contains("Unsupported Git credential type"));
            assert!(!error.message().contains("private-token"));
        }
    }
}
