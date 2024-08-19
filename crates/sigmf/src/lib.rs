use core::fmt;
use serde_json;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use tar::{Archive, Entry};

pub mod capture;
pub mod collection;
pub mod data;
pub mod global;
pub mod meta;
pub mod recording;
pub mod utils;

pub type Number = f64;
pub type Integer = u64;

#[derive(Debug, Default)]
pub struct SigMF {
    pub collection: Option<collection::SmfCollection>,
    pub recordings: Vec<recording::SmfRecording>,
    pub path: Option<String>,
}

pub fn read_file_as_bytes_in_archive(
    path: &str,
    name: &str,
    ext: &str,
) -> Result<Vec<u8>, SmfError> {
    let file = File::open(path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let file_path = file.path()?.to_path_buf();

        if let (Some(file_name), Some(file_ext)) = (
            file_path.file_stem().and_then(|s| s.to_str()),
            file_path.extension().and_then(|s| s.to_str()),
        ) {
            if file_name == name && file_ext == ext {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                return Ok(buffer);
            }
        }
    }

    Err(SmfError::NotFound("File not found".to_string()))
}

impl SigMF {
    pub fn new(
        collection: Option<collection::SmfCollection>,
        recordings: Vec<recording::SmfRecording>,
        path: Option<String>,
    ) -> Self {
        SigMF {
            collection,
            recordings,
            path,
        }
    }

    pub fn load_from_file(path: &str) -> Result<Self, SmfError> {
        assert!(path.ends_with(".sigmf"), "File must have .sigmf extension");
        let mut sigmf = SigMF::default();
        sigmf.path = Some(path.to_string());

        // Read the tarball
        let file = File::open(path)?;
        let mut archive = Archive::new(file);

        for file in archive.entries()? {
            let mut file = file?;
            let file_path = file.path()?.to_path_buf();

            if let (Some(file_name), Some(file_ext)) = (
                file_path.file_stem().and_then(|s| s.to_str()),
                file_path.extension().and_then(|s| s.to_str()),
            ) {
                match file_ext {
                    "sigmf-meta" => {
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)?;

                        let meta_hash = utils::hash_from_slice(&buffer[..]);

                        match meta_hash {
                            Ok(hash) => {
                                let recording = recording::SmfRecording {
                                    name: file_name.to_string(),
                                    hash,
                                };
                                sigmf.recordings.push(recording);
                            }
                            Err(_) => {
                                println!("Warning: Skipping file: {} with invalid hash", file_name);
                                println!("Error: {}", meta_hash.unwrap_err());
                                continue;
                            }
                        }
                    }
                    "sigmf-collection" => unimplemented!(),
                    _ => continue,
                }
            } else {
                println!(
                    "Warning: Skipping file {} with invalid extension",
                    file.path()?.display()
                );
                continue;
            }
        }

        Ok(sigmf)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), SmfError> {
        unimplemented!()
    }

    pub fn get_recordings(&self) -> Vec<recording::SmfRecording> {
        self.recordings.clone()
    }

    pub fn get_collection(&self) -> Option<collection::SmfCollection> {
        self.collection.clone()
    }

    pub fn get_path(&self) -> Option<String> {
        self.path.clone()
    }

    pub fn set_recordings(&mut self, recordings: Vec<recording::SmfRecording>) {
        self.recordings = recordings;
    }

    pub fn set_collection(&mut self, collection: collection::SmfCollection) {
        self.collection = Some(collection);
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Some(path);
    }

    pub fn get_recording_by_name(&self, name: &str) -> Option<recording::SmfRecording> {
        self.recordings
            .iter()
            .find(|recording| recording.name == name)
            .cloned()
    }

    pub fn get_meta_by_name(&self, name: &str) -> Result<meta::SmfMeta, SmfError> {
        match (&self).get_recording_by_name(name) {
            Some(recording) => match &self.path {
                Some(path) => {
                    // let mut buffer = Vec::new();
                    // file.read_to_end(&mut buffer)?;
                    // let meta_hash = utils::hash_from_slice(&buffer[..])?;

                    // if recording.hash == meta_hash {
                    //     let meta: meta::SmfMeta = serde_json::from_slice(&buffer)?;
                    //     return Ok(meta);
                    // }
                    Ok(meta::SmfMeta::default())
                }
                None => return Err(SmfError::NotFound("Path not found".to_string())),
            },
            None => Err(SmfError::NotFound("Recording not found".to_string())),
        }
    }
}

#[derive(Debug)]
pub enum SmfError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    InvalidDataType(String),
    NotFound(String),
}

impl From<std::io::Error> for SmfError {
    fn from(error: std::io::Error) -> Self {
        SmfError::Io(error)
    }
}

impl From<serde_json::Error> for SmfError {
    fn from(error: serde_json::Error) -> Self {
        SmfError::Serde(error)
    }
}

impl std::fmt::Display for SmfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SmfError::Io(err) => write!(f, "IO error: {}", err),
            SmfError::Serde(err) => write!(f, "Serde error: {}", err),
            SmfError::InvalidDataType(err) => write!(f, "Invalid data type{}", err),
            SmfError::NotFound(err) => write!(f, "Not found: {}", err),
        }
    }
}

impl Error for SmfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SmfError::Io(err) => Some(err),
            SmfError::Serde(err) => Some(err),
            SmfError::InvalidDataType(err) => None,
            SmfError::NotFound(err) => None,
        }
    }
}
