use std::env;

pub(crate) trait Pipeline {
    fn branch_name(&self) -> String;
    fn short_commit_sha(&self) -> String;
}

const GITLAB_CI: &str = "GITLAB_CI";
const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";

pub(crate) fn pipeline_info() -> (String, String) {
    if env::var(GITLAB_CI).map_or(false, |v| v == "true") {
        let gitlab_ci = crate::gitlab_ci::GitlabCI {};
        (gitlab_ci.branch_name(), gitlab_ci.short_commit_sha())
    } else if env::var(GITHUB_ACTIONS).map_or(false, |v| v == "true") {
        let github_action = crate::github_action::GithubAction {};
        (
            github_action.branch_name(),
            github_action.short_commit_sha(),
        )
    } else {
        let git_repo = crate::git_repo::GitRepo {};
        (git_repo.branch_name(), git_repo.short_commit_sha())
    }
}
