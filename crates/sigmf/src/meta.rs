use super::{capture, global, utils::hash::hash_from_slice, Integer, Number};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SmfMeta {
    global: global::SmfGlobal,
    annotations: Vec<SmfAnnotation>,
    captures: Vec<capture::SmfCapture>,
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
    // ================================== Methods ================================== //

    pub fn hash(&self) -> String {
        let serialized = serde_json::to_string(&self).unwrap();
        let hash = hash_from_slice(serialized.as_bytes()).unwrap();
        hash
    }

    // ================================== Setters ================================== //
    // ---------------- Global ---------------- //
    pub fn set_version(&mut self, version: String) {
        self.global.set_version(version);
    }

    pub fn set_datatype(&mut self, datatype: global::SmfDataType) {
        self.global.set_datatype(datatype);
    }

    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.global.set_sample_rate(sample_rate);
    }

    pub fn set_author(&mut self, author: String) {
        self.global.set_author(author);
    }

    pub fn set_collection(&mut self, collection_str: String) {
        self.global.set_collection(collection_str);
    }

    pub fn set_data_doi(&mut self, data_doi: String) {
        self.global.set_data_doi(data_doi);
    }

    pub fn set_description(&mut self, description: String) {
        self.global.set_description(description);
    }

    pub fn set_hw(&mut self, hw: String) {
        self.global.set_hw(hw);
    }

    pub fn set_license(&mut self, license: String) {
        self.global.set_license(license);
    }

    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.global.set_metadata_only(metadata_only);
    }

    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.global.set_meta_doi(meta_doi);
    }

    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.global.set_num_channels(num_channels);
    }

    pub fn set_offset(&mut self, offset: Integer) {
        self.global.set_offset(offset);
    }

    pub fn set_recorder(&mut self, recorder: String) {
        self.global.set_recorder(recorder);
    }

    pub fn set_sha512(&mut self, hash: String) {
        self.global.set_sha512(hash);
    }

    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.global.set_geolocation(geolocation);
    }

    pub fn set_extensions(&mut self, extensions: Vec<global::SmfExtension>) {
        self.global.set_extensions(extensions);
    }

    pub fn set_dataset(&mut self, dataset: String) {
        self.global.set_dataset(dataset);
    }

    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.global.set_trailing_bytes(trailing_bytes);
    }

    // ---------------- Annotations ---------------- //
    pub fn add_annotation(&mut self, annotation: SmfAnnotation) {
        self.annotations.push(annotation);
    }

    // ----------------- Captures ------------------ //
    pub fn add_capture(&mut self, capture: capture::SmfCapture) {
        self.captures.push(capture);
    }

    // ================================== Getters ================================== //
    // ---------------- Global ---------------- //
    pub fn get_version(&self) -> String {
        self.global.get_version()
    }

    pub fn get_datatype(&self) -> global::SmfDataType {
        self.global.get_datatype()
    }

    pub fn get_sample_rate(&self) -> Option<Number> {
        self.global.get_sample_rate()
    }

    pub fn get_author(&self) -> Option<String> {
        self.global.get_author()
    }

    pub fn get_collection(&self) -> Option<String> {
        self.global.get_collection()
    }

    pub fn get_data_doi(&self) -> Option<String> {
        self.global.get_data_doi()
    }

    pub fn get_description(&self) -> Option<String> {
        self.global.get_description()
    }

    pub fn get_hw(&self) -> Option<String> {
        self.global.get_hw()
    }

    pub fn get_license(&self) -> Option<String> {
        self.global.get_license()
    }

    pub fn get_metadata_only(&self) -> Option<bool> {
        self.global.get_metadata_only()
    }

    pub fn get_meta_doi(&self) -> Option<String> {
        self.global.get_meta_doi()
    }

    pub fn get_num_channels(&self) -> Option<Integer> {
        self.global.get_num_channels()
    }

    pub fn get_offset(&self) -> Option<Integer> {
        self.global.get_offset()
    }

    pub fn get_recorder(&self) -> Option<String> {
        self.global.get_recorder()
    }

    pub fn get_sha512(&self) -> Option<String> {
        self.global.get_sha512()
    }

    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.global.get_geolocation()
    }

    pub fn get_extensions(&self) -> Option<Vec<global::SmfExtension>> {
        self.global.get_extensions()
    }

    pub fn get_dataset(&self) -> Option<String> {
        self.global.get_dataset()
    }

    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.global.get_trailing_bytes()
    }

    // ---------------- Annotations ---------------- //
    pub fn get_annotations(&self) -> Vec<SmfAnnotation> {
        self.annotations.clone()
    }

    // ----------------- Captures ------------------ //
    pub fn get_captures(&self) -> Vec<capture::SmfCapture> {
        self.captures.clone()
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
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

impl SmfAnnotation {
    pub fn new(
        sample_start: Integer,
        sample_count: Option<Integer>,
        freq_lower_edge: Option<Number>,
        freq_upper_edge: Option<Number>,
        label: Option<String>,
        comment: Option<String>,
        generator: Option<String>,
        uuid: Option<String>,
    ) -> Self {
        SmfAnnotation {
            sample_start,
            sample_count,
            freq_lower_edge,
            freq_upper_edge,
            label,
            comment,
            generator,
            uuid,
        }
    }

    // Setters
    pub fn set_sample_start(&mut self, sample_start: Integer) {
        self.sample_start = sample_start;
    }

    pub fn set_sample_count(&mut self, sample_count: Integer) {
        self.sample_count = Some(sample_count);
    }

    pub fn set_freq_lower_edge(&mut self, freq_lower_edge: Number) {
        self.freq_lower_edge = Some(freq_lower_edge);
    }

    pub fn set_freq_upper_edge(&mut self, freq_upper_edge: Number) {
        self.freq_upper_edge = Some(freq_upper_edge);
    }

    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    pub fn set_comment(&mut self, comment: String) {
        self.comment = Some(comment);
    }

    pub fn set_generator(&mut self, generator: String) {
        self.generator = Some(generator);
    }

    pub fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid);
    }

    // Getters
    pub fn get_sample_start(&self) -> Integer {
        self.sample_start
    }

    pub fn get_sample_count(&self) -> Option<Integer> {
        self.sample_count
    }

    pub fn get_freq_lower_edge(&self) -> Option<Number> {
        self.freq_lower_edge
    }

    pub fn get_freq_upper_edge(&self) -> Option<Number> {
        self.freq_upper_edge
    }

    pub fn get_label(&self) -> Option<String> {
        self.label.clone()
    }

    pub fn get_comment(&self) -> Option<String> {
        self.comment.clone()
    }

    pub fn get_generator(&self) -> Option<String> {
        self.generator.clone()
    }

    pub fn get_uuid(&self) -> Option<String> {
        self.uuid.clone()
    }
}
