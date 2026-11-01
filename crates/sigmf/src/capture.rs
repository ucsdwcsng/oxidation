use super::{Integer, Number};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Capture {
    // Required fields
    #[serde(rename = "core:sample_start")]
    pub sample_start: Integer,

    // Optional fields
    #[serde(rename = "core:datetime", skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    #[serde(rename = "core:frequency", skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Number>,
    #[serde(rename = "core:global_index", skip_serializing_if = "Option::is_none")]
    pub global_index: Option<Integer>,
    #[serde(rename = "core:header_bytes", skip_serializing_if = "Option::is_none")]
    pub header_bytes: Option<Integer>,
    #[serde(rename = "core:geolocation", skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<GeoJson>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Capture {
    pub fn new(sample_start: Integer) -> Self {
        Capture {
            sample_start,
            datetime: None,
            frequency: None,
            global_index: None,
            header_bytes: None,
            geolocation: None,
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
            "core:sample_start"
                | "core:datetime"
                | "core:frequency"
                | "core:global_index"
                | "core:header_bytes"
                | "core:geolocation"
        ) {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    // Setters
    pub fn set_datetime(&mut self, datetime: String) {
        self.datetime = Some(datetime);
    }

    pub fn set_frequency(&mut self, frequency: Number) {
        self.frequency = Some(frequency);
    }

    pub fn set_global_index(&mut self, global_index: Integer) {
        self.global_index = Some(global_index);
    }

    pub fn set_header_bytes(&mut self, header_bytes: Integer) {
        self.header_bytes = Some(header_bytes);
    }

    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.geolocation = Some(geolocation);
    }

    // Getters
    pub fn get_sample_start(&self) -> Integer {
        self.sample_start
    }

    pub fn get_datetime(&self) -> Option<&String> {
        self.datetime.as_ref()
    }

    pub fn get_frequency(&self) -> Option<&Number> {
        self.frequency.as_ref()
    }

    pub fn get_global_index(&self) -> Option<&Integer> {
        self.global_index.as_ref()
    }

    pub fn get_header_bytes(&self) -> Option<&Integer> {
        self.header_bytes.as_ref()
    }
}
