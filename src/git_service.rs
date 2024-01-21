use git2::{Error, Repository};
use regex::Regex;

const SEMANTIC_VERSION_TAG_PATTERN: &str = r"^v?([0-9]+\.[0-9]+\.[0-9]+)$";

fn open_repo() -> Repository {
    Repository::open(".").unwrap_or_else(|e| panic!("Failed to open git repo: {}", e))
}

pub(crate) fn last_semantic_version_tag(default: String) -> String {
    let repo = open_repo();

    let semantic_version_regex = Regex::new(SEMANTIC_VERSION_TAG_PATTERN).unwrap();

    let tag_names = match repo.tag_names(None) {
        Ok(tag_names) => tag_names,
        Err(e) => {
            eprintln!("Failed to retrieve tags: {}", e);
            return default;
        }
    };

    tag_names
        .iter()
        .flatten()
        .filter(|t| semantic_version_regex.is_match(t))
        .last()
        .map_or(default, |tag_name| tag_name.to_string())
}

pub(crate) fn branch_name() -> Result<String, Error> {
    let repo = open_repo();

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
