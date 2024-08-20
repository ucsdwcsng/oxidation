use super::{capture, global, utils::hash::hash_from_slice, Integer, Number};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SmfMeta {
    pub global: global::SmfGlobal,
    pub annotations: Vec<SmfAnnotation>,
    pub captures: Vec<capture::SmfCapture>,
}

impl SmfMeta {
    pub fn new(
        global: global::SmfGlobal,
        annotations: Vec<SmfAnnotation>,
        captures: Vec<capture::SmfCapture>,
    ) -> Self {
        SmfMeta {
            global,
            annotations,
            captures,
        }
    }

    // Setters
    pub fn set_version(&mut self, version: String) {
        self.global.version = version;
    }

    pub fn set_datatype(&mut self, datatype: global::SmfDataType) {
        self.global.datatype = datatype;
    }

    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.global.sample_rate = Some(sample_rate);
    }

    pub fn set_author(&mut self, author: String) {
        self.global.author = Some(author);
    }

    pub fn set_collection(&mut self, collection_str: String) {
        self.global.collection = Some(collection_str);
    }

    pub fn set_data_doi(&mut self, data_doi: String) {
        self.global.data_doi = Some(data_doi);
    }

    pub fn set_description(&mut self, description: String) {
        self.global.description = Some(description);
    }

    pub fn set_hw(&mut self, hw: String) {
        self.global.hw = Some(hw);
    }

    pub fn set_license(&mut self, license: String) {
        self.global.license = Some(license);
    }

    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.global.metadata_only = Some(metadata_only);
    }

    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.global.meta_doi = Some(meta_doi);
    }

    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.global.num_channels = Some(num_channels);
    }

    pub fn set_offset(&mut self, offset: Integer) {
        self.global.offset = Some(offset);
    }

    pub fn set_recorder(&mut self, recorder: String) {
        self.global.recorder = Some(recorder);
    }

    pub fn set_sha512(&mut self, hash: String) {
        self.global.sha512 = Some(hash);
    }

    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.global.geolocation = Some(geolocation);
    }

    pub fn set_extensions(&mut self, extensions: Vec<global::SmfExtension>) {
        self.global.extensions = Some(extensions);
    }

    pub fn set_dataset(&mut self, dataset: String) {
        self.global.dataset = Some(dataset);
    }

    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.global.trailing_bytes = Some(trailing_bytes);
    }

    // Getters
    pub fn get_version(&self) -> String {
        self.global.version.clone()
    }

    pub fn get_datatype(&self) -> global::SmfDataType {
        self.global.datatype.clone()
    }

    pub fn get_sample_rate(&self) -> Option<Number> {
        self.global.sample_rate
    }

    pub fn get_author(&self) -> Option<String> {
        self.global.author.clone()
    }

    pub fn get_collection(&self) -> Option<String> {
        self.global.collection.clone()
    }

    pub fn get_data_doi(&self) -> Option<String> {
        self.global.data_doi.clone()
    }

    pub fn get_description(&self) -> Option<String> {
        self.global.description.clone()
    }

    pub fn get_hw(&self) -> Option<String> {
        self.global.hw.clone()
    }

    pub fn get_license(&self) -> Option<String> {
        self.global.license.clone()
    }

    pub fn get_metadata_only(&self) -> Option<bool> {
        self.global.metadata_only
    }

    pub fn get_meta_doi(&self) -> Option<String> {
        self.global.meta_doi.clone()
    }

    pub fn get_num_channels(&self) -> Option<Integer> {
        self.global.num_channels
    }

    pub fn get_offset(&self) -> Option<Integer> {
        self.global.offset
    }

    pub fn get_recorder(&self) -> Option<String> {
        self.global.recorder.clone()
    }

    pub fn get_sha512(&self) -> Option<String> {
        self.global.sha512.clone()
    }

    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.global.geolocation.clone()
    }

    pub fn get_extensions(&self) -> Option<Vec<global::SmfExtension>> {
        self.global.extensions.clone()
    }

    pub fn get_dataset(&self) -> Option<String> {
        self.global.dataset.clone()
    }

    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.global.trailing_bytes
    }

    pub fn hash(&self) -> String {
        let serialized = serde_json::to_string(&self).unwrap();
        let hash = hash_from_slice(serialized.as_bytes()).unwrap();
        hash
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SmfAnnotation {
    pub sample_start: Integer,
    pub sample_count: Option<Integer>,
    pub freq_lower_edge: Option<Number>,
    pub freq_upper_edge: Option<Number>,
    pub label: Option<String>,
    pub comment: Option<String>,
    pub generator: Option<String>,
    pub uuid: Option<String>,
}
