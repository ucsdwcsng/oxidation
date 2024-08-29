#![allow(unused_imports)]
use super::{Integer, Number};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SmfCapture {
    // Required fields
    #[serde(rename = "core:sample_start")]
    pub sample_start: Integer,

    // Optional fields
    #[serde(rename = "core:datetime")]
    pub datetime: Option<String>,
    #[serde(rename = "core:frequency")]
    pub frequency: Option<Number>,
    #[serde(rename = "core:global_index")]
    pub global_index: Option<Integer>,
    #[serde(rename = "core:header_bytes")]
    pub header_bytes: Option<Integer>,
}

impl SmfCapture {
    pub fn new(sample_start: Integer) -> Self {
        SmfCapture {
            sample_start,
            datetime: None,
            frequency: None,
            global_index: None,
            header_bytes: None,
        }
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
