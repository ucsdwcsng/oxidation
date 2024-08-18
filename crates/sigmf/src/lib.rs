use core::fmt;
use meta::SmfMeta;
use serde_derive::Deserialize;
use serde_json;
use sha2::Digest;
use std::error::Error;
use std::io::Read;

pub mod capture;
pub mod collection;
pub mod data;
pub mod global;
pub mod meta;

pub type Number = f64;
pub type Integer = u64;

#[derive(Debug, Default, Deserialize)]
pub struct SigMF {
    pub collection: Option<collection::SmfCollection>,
    pub recordings: Vec<SmfRecording>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SmfRecording {
    pub name: String,
    pub hash: String,
}

#[derive(Debug)]
pub enum SmfError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    InvalidDataType(Box<dyn Error>),
}

impl std::fmt::Display for SmfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SmfError::Io(err) => write!(f, "IO error: {}", err),
            SmfError::Serde(err) => write!(f, "Serde error: {}", err),
            SmfError::InvalidDataType(err) => write!(f, "Invalid data type{}", err),
        }
    }
}

impl Error for SmfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SmfError::Io(err) => Some(err),
            SmfError::Serde(err) => Some(err),
            SmfError::InvalidDataType(err) => None,
        }
    }
}

impl SigMF {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(SigMF {
            collection: None,
            recordings: Vec::new(),
        })
    }
}

impl SmfRecording {
    pub fn new(
        metadata_file_path: &String,
        data_file_path: &String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let metadata_file = std::fs::File::open(&metadata_file_path)?;
        let metadata_reader = std::io::BufReader::new(metadata_file);
        let metadata: SmfMeta = serde_json::from_reader(metadata_reader)?;

        println!("{:?}", metadata);

        let data_file = std::fs::File::open(&data_file_path)?;
        let data_reader = std::io::BufReader::new(data_file);
        let mut data: Vec<u8> = Vec::new();
        data_reader.take(1000).read_to_end(&mut data)?;

        let mut hasher = sha2::Sha512::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        let hash_str = format!("{:x}", hash);

        let recording_name = &data_file_path.replace(".sigmf-data", "");

        let recording = SmfRecording {
            name: recording_name.to_string(),
            hash: hash_str,
        };
        Ok(recording)
    }
}
