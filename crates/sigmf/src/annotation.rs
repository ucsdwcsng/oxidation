use super::{Integer, Number};
use serde_derive::{Deserialize, Serialize};

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
