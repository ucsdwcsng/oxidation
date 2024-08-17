use serde_derive::Deserialize;

use super::{global::SmfExtension, SmfRecording};

#[derive(Debug, Default, Deserialize)]
pub struct SmfCollection {
    // Required fields
    pub version: String,

    // Optional fields
    pub description: Option<String>,
    pub author: Option<String>,
    pub collection_doi: Option<String>,
    pub license: Option<String>,
    pub extensions: Option<Vec<SmfExtension>>,
    pub streams: Option<Vec<SmfRecording>>,
}
