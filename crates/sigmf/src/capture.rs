#![allow(unused_imports)]
use super::{Integer, Number};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct SmfCapture {
    pub capture_list: Vec<SmfCaptureList>,
}

#[derive(Debug, Deserialize)]
pub struct SmfCaptureList {
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
