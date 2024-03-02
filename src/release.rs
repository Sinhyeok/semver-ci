use crate::pipelines;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct Release {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) tag_name: String,
    pub(crate) tag_message: String,
}

impl Release {
    pub(crate) fn create(&self) -> HashMap<String, Value> {
        pipelines::create_release(self)
    }
}
