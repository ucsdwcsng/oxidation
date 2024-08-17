use super::{capture, global, Integer, Number};
use serde_derive::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct SmfMeta {
    pub global: global::SmfGlobal,
    pub annotations: Vec<SmfAnnotation>,
    pub captures: Vec<capture::SmfCapture>,
}

#[derive(Debug, Default, Deserialize)]
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
