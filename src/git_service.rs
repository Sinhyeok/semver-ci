use git2::{Error, Repository};
use regex::Regex;

pub(crate) fn open_repo() -> Result<Repository, Error> {
    Repository::open(".")
}

pub(crate) fn latest_semantic_version_tag(repo: &Repository) -> Option<String> {
    let semantic_version_regex = Regex::new(r"^v?(\d+\.\d+\.\d+)$").unwrap();

    let tag_names = match repo.tag_names(None) {
        Ok(tag_names) => tag_names,
        Err(e) => {
            eprintln!("Failed to retrieve tags: {}", e);
            return None;
        }
    };

    let mut semantic_version_tag_names: Vec<_> = tag_names
        .into_iter()
        .filter(|t| semantic_version_regex.is_match(t.unwrap()))
        .collect();

    semantic_version_tag_names.sort();

    match semantic_version_tag_names.into_iter().last() {
        Some(latest_tag_name) => Some(latest_tag_name?.to_string()),
        None => None,
    }
}
