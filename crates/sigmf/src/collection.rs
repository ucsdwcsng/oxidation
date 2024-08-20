use serde_derive::{Deserialize, Serialize};

use super::{global::SmfExtension, recording::SmfRecording};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SmfCollection {
    pub collection: SmfCollectionEntry,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SmfCollectionEntry {
    // Required fields
    #[serde(rename = "core:version")]
    pub version: String,

    // Optional fields
    #[serde(rename = "core:description")]
    pub description: Option<String>,
    #[serde(rename = "core:author")]
    pub author: Option<String>,
    #[serde(rename = "core:collection_doi")]
    pub collection_doi: Option<String>,
    #[serde(rename = "core:license")]
    pub license: Option<String>,
    #[serde(rename = "core:extensions")]
    pub extensions: Option<Vec<SmfExtension>>,
    #[serde(rename = "core:streams")]
    pub streams: Option<Vec<SmfRecording>>,
}
