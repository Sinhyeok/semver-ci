use git2::{Error, Repository};
use regex::Regex;

pub(crate) fn open_repo() -> Result<Repository, Error> {
    Repository::open(".")
}

pub(crate) fn latest_semantic_version_tag(repo: &Repository) -> Option<String> {
    let semantic_version_regex = Regex::new(r"^v?([0-9]+\.[0-9]+\.[0-9]+)$").unwrap();

    let tag_names = match repo.tag_names(None) {
        Ok(tag_names) => tag_names,
        Err(e) => {
            eprintln!("Failed to retrieve tags: {}", e);
            return None;
        }
    };

    tag_names
        .iter()
        .flatten()
        .filter(|t| semantic_version_regex.is_match(t))
        .last()
        .map(|tag_name| tag_name.to_string())
}
