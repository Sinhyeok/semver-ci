use git2::Repository;
use regex::Regex;

pub(crate) fn last_semantic_version_tag() -> Option<String> {
    let repo = Repository::open(".").unwrap_or_else(|e| panic!("Failed to open git repo: {}", e));

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
