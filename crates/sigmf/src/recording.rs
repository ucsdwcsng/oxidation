use crate::utils::get_hash_from;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SmfRecording {
    pub name: String,
    pub hash: String,
}

impl SmfRecording {
    pub fn new(name: String, hash: String) -> Self {
        SmfRecording { name, hash }
    }

    pub fn from_path(metafile_path: &String) -> Result<Self, Box<dyn std::error::Error>> {
        let hash_str = get_hash_from(&metafile_path)?;

        let recording_name = &metafile_path.replace(".sigmf-meta", "");
        let recording = SmfRecording {
            name: recording_name.to_string(),
            hash: hash_str,
        };
        Ok(recording)
    }
}
