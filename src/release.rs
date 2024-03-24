pub(crate) struct Release {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) tag_name: String,
    pub(crate) tag_message: String,
    pub(crate) generate_release_notes: bool,
    pub(crate) previous_tag: String,
}
