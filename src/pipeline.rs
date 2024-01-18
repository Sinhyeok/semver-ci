pub(crate) trait Pipeline {
    fn branch_name(&self) -> String;
    fn short_commit_sha(&self) -> String;
}
