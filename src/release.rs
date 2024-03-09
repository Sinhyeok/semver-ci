use crate::pipelines;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct Release {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) tag_name: String,
    pub(crate) tag_message: String,
    pub(crate) generate_release_notes: bool,
    pub(crate) previous_tag: String,
}

impl Release {
    pub(crate) fn create(&self) -> HashMap<String, Value> {
        let pipeline = pipelines::current_pipeline();
        pipeline.create_release(self)
    }
}
