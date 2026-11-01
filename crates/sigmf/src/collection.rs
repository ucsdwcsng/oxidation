use serde::de::{self, Deserializer};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{global::ExtensionDeclaration, recording::RecordingRef};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Collection {
    pub collection: CollectionMetadata,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollectionMetadata {
    // Required fields
    #[serde(rename = "core:version")]
    pub version: String,

    // Optional fields
    #[serde(rename = "core:description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "core:author", skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(
        rename = "core:collection_doi",
        skip_serializing_if = "Option::is_none"
    )]
    pub collection_doi: Option<String>,
    #[serde(rename = "core:license", skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(rename = "core:extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<ExtensionDeclaration>>,
    #[serde(rename = "core:streams", deserialize_with = "deserialize_streams")]
    pub streams: Vec<RecordingRef>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Collection {
    pub fn new(collection: CollectionMetadata) -> Self {
        Self {
            collection,
            extra_fields: Map::new(),
        }
    }

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, crate::SigmfError> {
        if bytes.len() > crate::MAX_META_SIZE {
            return Err(crate::SigmfError::ResourceLimit(format!(
                "collection metadata exceeds the {} byte limit",
                crate::MAX_META_SIZE
            )));
        }
        let value = crate::json::parse_unique_value(bytes)?;
        let root = value.as_object().ok_or_else(|| {
            crate::SigmfError::InvalidMetadata("expected collection object at /".to_string())
        })?;
        let collection = root
            .get("collection")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                crate::SigmfError::InvalidMetadata(
                    "expected collection object at /collection".to_string(),
                )
            })?;
        for required in ["core:version", "core:streams"] {
            if !collection.contains_key(required) {
                return Err(crate::SigmfError::InvalidMetadata(format!(
                    "missing required collection field at /collection/{required}"
                )));
            }
        }
        if !collection["core:version"].is_string() {
            return Err(crate::SigmfError::InvalidMetadata(
                "expected a string at /collection/core:version".into(),
            ));
        }
        let Some(streams) = collection["core:streams"].as_array() else {
            return Err(crate::SigmfError::InvalidMetadata(
                "expected an array at /collection/core:streams".into(),
            ));
        };
        for (index, stream) in streams.iter().enumerate() {
            let pointer = format!("/collection/core:streams/{index}");
            match stream {
                Value::Array(tuple) => {
                    if tuple.len() != 2 || !tuple[0].is_string() || !tuple[1].is_string() {
                        return Err(crate::SigmfError::InvalidMetadata(format!(
                            "expected a two-string legacy tuple at {pointer}"
                        )));
                    }
                }
                Value::Object(object) => {
                    for field in ["name", "hash"] {
                        if !object.get(field).is_some_and(Value::is_string) {
                            return Err(crate::SigmfError::InvalidMetadata(format!(
                                "expected a string at {pointer}/{field}"
                            )));
                        }
                    }
                }
                _ => {
                    return Err(crate::SigmfError::InvalidMetadata(format!(
                        "expected an object or legacy tuple at {pointer}"
                    )));
                }
            }
        }
        for field in [
            "core:description",
            "core:author",
            "core:collection_doi",
            "core:license",
            "core:extensions",
        ] {
            if collection.get(field).is_some_and(Value::is_null) {
                return Err(crate::SigmfError::InvalidMetadata(format!(
                    "null is not permitted at /collection/{field}; omit the field instead"
                )));
            }
        }
        serde_json::from_value(value)
            .map_err(|error| crate::SigmfError::InvalidMetadata(error.to_string()))
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, crate::SigmfError> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn to_vec_preserving_stream_forms(&self) -> Result<Vec<u8>, crate::SigmfError> {
        let mut value = serde_json::to_value(self)?;
        let streams = value
            .get_mut("collection")
            .and_then(Value::as_object_mut)
            .and_then(|collection| collection.get_mut("core:streams"))
            .and_then(Value::as_array_mut)
            .ok_or_else(|| {
                crate::SigmfError::InvalidMetadata(
                    "serialized collection is missing /collection/core:streams".into(),
                )
            })?;
        for (reference, serialized) in self.collection.streams.iter().zip(streams) {
            if reference.was_legacy_tuple() {
                *serialized = Value::Array(vec![
                    Value::String(reference.get_name().to_owned()),
                    Value::String(reference.get_hash().to_owned()),
                ]);
            }
        }
        Ok(serde_json::to_vec(&value)?)
    }

    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if key == "collection" {
            return Err(crate::SigmfError::InvalidMetadata(
                "cannot insert known field \"collection\" as an extension".into(),
            ));
        }
        Ok(self.extra_fields.insert(key, value))
    }
}

impl CollectionMetadata {
    pub fn new(version: impl Into<String>, streams: Vec<RecordingRef>) -> Self {
        Self {
            version: version.into(),
            description: None,
            author: None,
            collection_doi: None,
            license: None,
            extensions: None,
            streams,
            extra_fields: Map::new(),
        }
    }

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn extension_fields_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extra_fields
    }

    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if matches!(
            key.as_str(),
            "core:version"
                | "core:description"
                | "core:author"
                | "core:collection_doi"
                | "core:license"
                | "core:extensions"
                | "core:streams"
        ) {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }
}

fn deserialize_streams<'de, D>(deserializer: D) -> Result<Vec<RecordingRef>, D::Error>
where
    D: Deserializer<'de>,
{
    let entries = <Vec<Value> as serde::Deserialize>::deserialize(deserializer)?;
    entries
        .into_iter()
        .map(|entry| {
            if let Some(tuple) = entry.as_array() {
                if tuple.len() != 2 {
                    return Err(de::Error::custom(
                        "legacy recording references must be exact two-element tuples",
                    ));
                }
                let name = tuple[0]
                    .as_str()
                    .ok_or_else(|| de::Error::custom("tuple recording name must be a string"))?;
                let hash = tuple[1]
                    .as_str()
                    .ok_or_else(|| de::Error::custom("tuple recording hash must be a string"))?;
                Ok(RecordingRef::from_legacy_tuple(name, hash))
            } else {
                serde_json::from_value(entry).map_err(de::Error::custom)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_legacy_tuples_and_writes_normalized_reference_objects() {
        let collection = Collection::from_slice(
            br#"{"collection":{"core:version":"1.2.6","core:streams":[["a","abcd"],{"name":"b","hash":"ef01","vendor:tag":1}]}}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&collection.to_vec().unwrap()).unwrap();
        assert_eq!(value["collection"]["core:streams"][0]["name"], "a");
        assert_eq!(value["collection"]["core:streams"][0]["hash"], "abcd");
        assert_eq!(value["collection"]["core:streams"][1]["vendor:tag"], 1);
        let preserved: Value =
            serde_json::from_slice(&collection.to_vec_preserving_stream_forms().unwrap()).unwrap();
        assert_eq!(
            preserved["collection"]["core:streams"][0],
            serde_json::json!(["a", "abcd"])
        );
        assert!(preserved["collection"]["core:streams"][1].is_object());
    }

    #[test]
    fn collection_parser_enforces_metadata_size_limit() {
        let oversized = vec![b' '; crate::MAX_META_SIZE + 1];
        assert!(matches!(
            Collection::from_slice(&oversized),
            Err(crate::SigmfError::ResourceLimit(_))
        ));
    }
}
