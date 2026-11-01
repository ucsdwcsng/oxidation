use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingRef {
    name: String,
    hash: String,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
    #[serde(skip)]
    legacy_tuple: bool,
}

impl RecordingRef {
    pub fn new(name: &str, hash: &str) -> Self {
        RecordingRef {
            name: name.to_string(),
            hash: hash.to_string(),
            extra_fields: Map::new(),
            legacy_tuple: false,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_hash(&self) -> &str {
        &self.hash
    }

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if matches!(key.as_str(), "name" | "hash") {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_hash(&mut self, hash: String) {
        self.hash = hash;
    }

    pub(crate) fn from_legacy_tuple(name: &str, hash: &str) -> Self {
        let mut reference = Self::new(name, hash);
        reference.legacy_tuple = true;
        reference
    }

    pub(crate) fn was_legacy_tuple(&self) -> bool {
        self.legacy_tuple
    }
}
