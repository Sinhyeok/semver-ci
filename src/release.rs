use crate::pipelines;
use crate::pipelines::Pipeline;

pub(crate) struct Release {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) tag_name: String,
    pub(crate) tag_message: String,
}

impl Release {
    pub(crate) fn create(&self) {
        match pipelines::pipelines() {
            pipelines::Pipelines::GithubActions(p) => p.create_release(self),
            pipelines::Pipelines::GitlabCI(p) => p.create_release(self),
            pipelines::Pipelines::GitRepo(p) => p.create_release(self),
        };
    }
}
