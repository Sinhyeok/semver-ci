use crate::semantic_version::SemanticVersion;
use crate::VersionArgs;

pub(crate) fn run(args: &VersionArgs) {
    let scope = args.scope.clone();
    // TODO: fn latest_tag() -> String
    let latest_tag = "v0.1.0".to_string();
    let version = version(scope, latest_tag);

    // TODO: fn branch_name() -> String
    let branch_name = "develop".to_string();
    // TODO: fn short_commit_sha() -> Option<String>
    let short_commit_sha = Some("ahs9df9d".to_string());
    let metadata = metadata(branch_name, short_commit_sha);

    println!("{}{}", version, metadata)
}

fn version(scope: String, latest_tag: String) -> String {
    let mut semantic_version = match SemanticVersion::from_string(latest_tag[1..].to_string()) {
        Ok(semantic_version) => semantic_version,
        Err(e) => {
            panic!("{}: {}", e, latest_tag)
        }
    };

    match scope.as_str() {
        "major" => semantic_version.increase_major(),
        "minor" => semantic_version.increase_minor(),
        "patch" => semantic_version.increase_patch(),
        _ => {
            panic!("Invalid scope: {}", scope)
        }
    }

    semantic_version.to_string(true)
}

fn metadata(branch_name: String, short_commit_sha: Option<String>) -> String {
    if branch_name == "develop" {
        match short_commit_sha {
            Some(short_commit_sha_ok) => {
                format!("-dev.{}", short_commit_sha_ok)
            }
            None => {
                panic!("Not found short_commit_sha on develop branch")
            }
        }
    } else {
        "".to_string()
    }
}
