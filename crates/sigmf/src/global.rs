use super::{Integer, Number, SigmfError};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use crate::datatype::{ByteType, ComponentType, Datatype, Endianness, SampleDomain};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Global {
    // Required fields
    #[serde(rename = "core:version")]
    pub version: String,
    #[serde(rename = "core:datatype")]
    pub datatype: Datatype,

    // Optional fields
    #[serde(rename = "core:sample_rate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<Number>,
    #[serde(rename = "core:author", skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "core:collection", skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(rename = "core:data_doi", skip_serializing_if = "Option::is_none")]
    pub data_doi: Option<String>,
    #[serde(rename = "core:description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "core:hw", skip_serializing_if = "Option::is_none")]
    pub hw: Option<String>,
    #[serde(rename = "core:license", skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(rename = "core:metadata_only", skip_serializing_if = "Option::is_none")]
    pub metadata_only: Option<bool>, // Cannot be used in Non-Conforming Datasets
    #[serde(rename = "core:meta_doi", skip_serializing_if = "Option::is_none")]
    pub meta_doi: Option<String>,
    #[serde(rename = "core:num_channels", skip_serializing_if = "Option::is_none")]
    pub num_channels: Option<Integer>,
    #[serde(rename = "core:offset", skip_serializing_if = "Option::is_none")]
    pub offset: Option<Integer>,
    #[serde(rename = "core:recorder", skip_serializing_if = "Option::is_none")]
    pub recorder: Option<String>,
    #[serde(rename = "core:sha512", skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(rename = "core:geolocation", skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<GeoJson>,
    #[serde(rename = "core:extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<ExtensionDeclaration>>,

    // For Non-Conforming Datasets ONLY
    #[serde(rename = "core:dataset", skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    #[serde(
        rename = "core:trailing_bytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub trailing_bytes: Option<Integer>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Global {
    pub fn new(version: impl Into<String>, datatype: Datatype) -> Self {
        Self {
            version: version.into(),
            datatype,
            sample_rate: None,
            author: None,
            collection: None,
            data_doi: None,
            description: None,
            hw: None,
            license: None,
            metadata_only: None,
            meta_doi: None,
            num_channels: None,
            offset: None,
            recorder: None,
            sha512: None,
            geolocation: None,
            extensions: None,
            dataset: None,
            trailing_bytes: None,
            extra_fields: Map::new(),
        }
    }

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn update_extension_fields(
        &mut self,
        known_fields: &[&str],
        fields: Map<String, Value>,
    ) {
        for field in known_fields {
            self.extra_fields.remove(*field);
        }
        self.extra_fields.extend(fields);
    }

    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, SigmfError> {
        let key = key.into();
        if is_core_global_key(&key) {
            return Err(SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    // Setters
    pub fn set_version(&mut self, version: String) {
        self.version = version;
    }

    pub fn set_datatype(&mut self, datatype: Datatype) {
        self.datatype = datatype;
    }

    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.sample_rate = Some(sample_rate);
    }

    pub fn set_author(&mut self, author: String) {
        self.author = Some(author);
    }

    pub fn set_collection(&mut self, collection_str: String) {
        self.collection = Some(collection_str);
    }

    pub fn set_data_doi(&mut self, data_doi: String) {
        self.data_doi = Some(data_doi);
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn set_hw(&mut self, hw: String) {
        self.hw = Some(hw);
    }

    pub fn set_license(&mut self, license: String) {
        self.license = Some(license);
    }

    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.metadata_only = Some(metadata_only);
    }

    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.meta_doi = Some(meta_doi);
    }

    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.num_channels = Some(num_channels);
    }

    pub fn set_offset(&mut self, offset: Integer) {
        self.offset = Some(offset);
    }

    pub fn set_recorder(&mut self, recorder: String) {
        self.recorder = Some(recorder);
    }

    pub fn set_sha512(&mut self, hash: String) {
        self.sha512 = Some(hash);
    }

    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.geolocation = Some(geolocation);
    }

    pub fn set_extensions(&mut self, extensions: Vec<ExtensionDeclaration>) {
        self.extensions = Some(extensions);
    }

    pub fn set_dataset(&mut self, dataset: String) {
        self.dataset = Some(dataset);
    }

    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.trailing_bytes = Some(trailing_bytes);
    }

    // Getters
    pub fn get_version(&self) -> String {
        self.version.clone()
    }

    pub fn get_datatype(&self) -> Datatype {
        self.datatype.clone()
    }

    pub fn get_sample_rate(&self) -> Option<Number> {
        self.sample_rate
    }

    pub fn get_author(&self) -> Option<String> {
        self.author.clone()
    }

    pub fn get_collection(&self) -> Option<String> {
        self.collection.clone()
    }

    pub fn get_data_doi(&self) -> Option<String> {
        self.data_doi.clone()
    }

    pub fn get_description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn get_hw(&self) -> Option<String> {
        self.hw.clone()
    }

    pub fn get_license(&self) -> Option<String> {
        self.license.clone()
    }

    pub fn get_metadata_only(&self) -> Option<bool> {
        self.metadata_only
    }

    pub fn get_meta_doi(&self) -> Option<String> {
        self.meta_doi.clone()
    }

    pub fn get_num_channels(&self) -> Option<Integer> {
        self.num_channels
    }

    pub fn get_offset(&self) -> Option<Integer> {
        self.offset
    }

    pub fn get_recorder(&self) -> Option<String> {
        self.recorder.clone()
    }

    pub fn get_sha512(&self) -> Option<String> {
        self.sha512.clone()
    }

    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.geolocation.clone()
    }

    pub fn get_extensions(&self) -> Option<Vec<ExtensionDeclaration>> {
        self.extensions.clone()
    }

    pub fn get_dataset(&self) -> Option<String> {
        self.dataset.clone()
    }

    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.trailing_bytes
    }
}

fn is_core_global_key(key: &str) -> bool {
    matches!(
        key,
        "core:version"
            | "core:datatype"
            | "core:sample_rate"
            | "core:author"
            | "core:collection"
            | "core:data_doi"
            | "core:description"
            | "core:hw"
            | "core:license"
            | "core:metadata_only"
            | "core:meta_doi"
            | "core:num_channels"
            | "core:offset"
            | "core:recorder"
            | "core:sha512"
            | "core:geolocation"
            | "core:extensions"
            | "core:dataset"
            | "core:trailing_bytes"
    )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtensionDeclaration {
    pub name: String,
    pub version: String,
    pub optional: bool,
}
