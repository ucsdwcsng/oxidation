use crate::utils::hash::get_hash_from;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SmfRecording {
    name: String,
    hash: String,
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

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_hash(&mut self, hash: String) {
        self.hash = hash;
    }
}
