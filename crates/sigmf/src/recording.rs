use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SmfRecording {
    name: String,
    hash: String,
}

impl SmfRecording {
    pub fn new(name: &str, hash: &str) -> Self {
        SmfRecording {
            name: name.to_string(),
            hash: hash.to_string(),
        }
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
