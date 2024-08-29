use core::fmt;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde_json;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tar::Archive;
use utils::data::DataSample;

pub mod annotation;
pub mod capture;
pub mod collection;
pub mod global;
pub mod meta;
pub mod recording;
pub mod utils;

use crate::recording::SmfRecording;
use crate::utils::hash::hash_from_path;

pub type Number = f64;
pub type Integer = u64;

const MAX_META_SIZE: usize = 1_000_000;

/// A [`SigMF`] object that holds the collection, recordings, and
/// path of the open .sigmf file
///
/// # Fields
/// * `collection` - An optional [`collection::SmfCollection`] object
/// * `recordings` - A vector of [`recording::SmfRecording`] objects
/// * `path` - An optional string slice that holds the path to the open .sigmf file
#[derive(Debug, Default)]
pub struct SigMF {
    pub collection: Option<collection::SmfCollection>,
    pub recordings: Vec<recording::SmfRecording>,
    pub archive_path: Option<String>,
}

impl SigMF {
    // ---------------------------------------- Constructors ---------------------------------------- //

    /// Create a new [`SigMF`] object using an optional path
    ///
    /// # Arguments
    /// * `path` - An optional string slice that holds the path to the .sigmf file
    ///
    /// # Returns
    /// A new [`SigMF`] object
    ///
    /// [`SigMF`]: crate::SigMF
    pub fn new(path: String) -> Self {
        SigMF {
            collection: None,
            recordings: vec![],
            archive_path: Some(path),
        }
    }

    // ------------------------------------------- Methods ------------------------------------------ //

    /// Save the [`SigMF`] object to a .sigmf file
    ///
    /// # Arguments
    /// * `path` - A string slice that holds the path to the .sigmf file
    ///
    /// # Returns
    /// A unit if successful, or a SmfError if an error occurred
    pub fn save(&self, save_path: &str) -> Result<(), SmfError> {
        if let Some(archive_path) = &self.archive_path {
            assert!(
                archive_path.ends_with(".sigmf") || archive_path.ends_with(".sigmf-meta"),
                "File must have .sigmf or .sigmf-meta extension"
            );

            let bar = ProgressBar::new(self.recordings.len() as u64);
            bar.set_style(
                ProgressStyle::with_template(
                    "Saving\t{elapsed_precise} {msg} {bar:40.cyan/blue} {pos:>7}/{len:7} {percent}%",
                )
                .unwrap(),
            );

            let file = File::create(save_path)?;
            let new_archive = Arc::new(Mutex::new(tar::Builder::new(file)));

            self.recordings.par_iter().try_for_each(|recording| {
                bar.inc(1);
                let mut new_archive_lock = new_archive
                    .lock()
                    .map_err(|_| SmfError::LockError("Error locking archive".to_string()))?;

                // Fetch metadata bytes from archive
                match utils::file::get_bytes_in_archive_from_hash(
                    archive_path,
                    &recording.get_hash(),
                ) {
                    Ok((meta_bytes, meta_header)) => {
                        new_archive_lock.append(&meta_header, &meta_bytes[..])?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                // Fetch data bytes from archive
                match utils::file::get_bytes_in_archive_from_name_ext(
                    archive_path,
                    &recording.get_name(),
                    "sigmf-data",
                ) {
                    Ok((data_bytes, data_header)) => {
                        new_archive_lock.append(&data_header, &data_bytes[..])?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }

                Ok(())
            })?;

            bar.finish();

            Ok(())
        } else {
            Err(SmfError::NotFound("Path is None".to_string()))
        }
    }

    /// Trim the [`SigMF`] object
    /// Removes all recordings that have less than min_samp_count samples
    ///
    /// # Arguments
    /// * `min_samp_count` - An integer that holds the minimum number of samples
    ///
    /// # Returns
    /// A unit if successful, or a SmfError if an error occurred
    pub fn trim(&mut self, min_samp_count: usize) -> Result<(), SmfError> {
        let bar = ProgressBar::new(self.recordings.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "Trimming\t{elapsed_precise} {msg} {bar:40.cyan/blue} {pos:>7}/{len:7} {percent}%",
            )
            .unwrap(),
        );

        // Use a Mutex to safely collect recordings in parallel
        let recordings = Arc::new(Mutex::new(Vec::new()));

        // Parallel processing of recordings
        self.recordings.par_iter().for_each(|recording| {
            // You should acquire the samples inside the scope of the for_each
            let samples = self
                .get_samples_from_recording(recording)
                .unwrap_or_else(|_| vec![]);
            if samples.len() >= min_samp_count {
                let mut recordings_guard = recordings.lock().unwrap();
                recordings_guard.push(recording.clone());
            }
            bar.inc(1); // Update progress bar
        });

        bar.finish();

        // Replace recordings with the collected results
        self.recordings = Arc::try_unwrap(recordings)
            .map_err(|_| SmfError::InvalidDataType("".to_string()))?
            .into_inner()
            .unwrap();
        Ok(())
    }

    // ------------------------------------------- Setters ------------------------------------------ //

    /// Set the recordings of the [`SigMF`] object
    ///
    /// # Arguments
    /// * `recordings` - A vector of [`recording::SmfRecording`] objects
    ///
    /// [`SigMF`]: crate::SigMF
    /// [`recording::SmfRecording`]: crate::recording::SmfRecording
    pub fn set_recordings(&mut self, recordings: Vec<recording::SmfRecording>) {
        self.recordings = recordings;
    }

    /// Set the collection of the [`SigMF`] object
    ///
    /// # Arguments
    /// * `collection` - A [`collection::SmfCollection`] object
    ///
    /// [`SigMF`]: crate::SigMF
    /// [`collection::SmfCollection`]: crate::collection::SmfCollection
    pub fn set_collection(&mut self, collection: collection::SmfCollection) {
        self.collection = Some(collection);
    }

    /// Set the path of the [`SigMF`] object
    ///
    /// # Arguments
    /// * `path` - A string slice that holds the path to the .sigmf file
    ///
    /// [`SigMF`]: crate::SigMF
    pub fn set_path(&mut self, path: &str) {
        self.archive_path = Some(path.to_string());
    }

    /// Add a recording to the [`SigMF`] object
    ///
    /// # Arguments
    /// * `recording` - A [`recording::SmfRecording`] object
    ///
    /// [`SigMF`]: crate::SigMF
    /// [`recording::SmfRecording`]: crate::recording::SmfRecording
    pub fn add_recording(&mut self, recording: recording::SmfRecording) {
        self.recordings.push(recording);
    }

    /// Add a collection to the [`SigMF`] object
    ///
    /// # Arguments
    /// * `collection` - A [`collection::SmfCollection`] object
    ///
    /// [`SigMF`]: crate::SigMF
    /// [`collection::SmfCollection`]: crate::collection::SmfCollection
    pub fn add_collection(&mut self, collection: collection::SmfCollection) {
        self.collection = Some(collection);
    }

    // ------------------------------------------- Getters ------------------------------------------ //

    /// Get the recordings of the [`SigMF`] object
    ///
    /// # Returns
    /// A vector of [`recording::SmfRecording`] objects
    pub fn get_recordings(&self) -> Vec<recording::SmfRecording> {
        self.recordings.clone()
    }

    /// get the collection of the [`SigMF`] object
    ///
    /// # Returns
    /// An optional [`collection::SmfCollection`] object
    pub fn get_collection(&self) -> Option<collection::SmfCollection> {
        self.collection.clone()
    }

    /// Get the path of the [`SigMF`] object
    ///
    /// # Returns
    /// An optional string slice that holds the path to the .sigmf file
    pub fn get_path(&self) -> Option<String> {
        self.archive_path.clone()
    }

    /// Get the names of the [`SmfRecording`]s
    ///
    /// # Returns
    /// A vector of string slices that hold the names of the [`SmfRecording`]s
    ///
    /// [`SmfRecording`]: crate::recording::SmfRecording
    pub fn get_names(&self) -> Vec<String> {
        self.recordings
            .par_iter()
            .map(|recording| recording.get_name())
            .collect()
    }

    /// Get a [`SmfRecording`] by name
    ///
    /// # Arguments
    /// * `name` - A string slice that holds the name of the [`SmfRecording`]
    ///
    /// # Returns
    /// An optional [`SmfRecording`] object
    ///
    /// [`SmfRecording`]: crate::recording::SmfRecording
    pub fn get_recording_from_name(&self, name: &str) -> Option<recording::SmfRecording> {
        self.recordings
            .par_iter()
            .find_any(|recording| recording.get_name() == name)
            .cloned()
    }

    /// Get the [`SmfMeta`] from a recording
    ///
    /// # Arguments
    /// * `recording` - A [`SmfRecording`] object
    ///
    /// [`SmfMeta`]: crate::meta::SmfMeta
    /// [`SmfRecording`]: crate::recording::SmfRecording
    pub fn get_meta_from_recording(
        &self,
        recording: &recording::SmfRecording,
    ) -> Result<meta::SmfMeta, SmfError> {
        match &self.archive_path {
            Some(path) => {
                // Get the meta data from the recording
                let (buffer, _) =
                    utils::file::get_bytes_in_archive_from_hash(path, &recording.get_hash())?;
                let meta: meta::SmfMeta = serde_json::from_slice(&buffer)?;
                Ok(meta)
            }
            None => match env::current_dir() {
                Ok(path) => {
                    let file_path =
                        format!("{}/{}.sigmf-meta", path.display(), recording.get_name());
                    let file = File::open(file_path)?;
                    let meta: meta::SmfMeta = serde_json::from_reader(file)?;
                    Ok(meta)
                }
                Err(e) => {
                    eprintln!("Error getting current directory: {}", e);
                    Err(SmfError::NotFound("Path is None".to_string()))
                }
            },
        }
    }

    /// Get the [`SmfMeta`] from a [`SmfRecording`] by name
    ///
    /// # Arguments
    /// * `name` - A string slice that holds the name of the [`SmfRecording`]
    ///
    /// # Returns
    /// A Result object with the [`SmfMeta`] object if successful, or a [`SmfError`] if an error occurred
    ///
    /// [`SmfMeta`]: crate::meta::SmfMeta
    /// [`SmfRecording`]: crate::recording::SmfRecording
    /// [`SmfError`]: crate::SmfError
    pub fn get_meta_from_name(&self, name: &str) -> Result<meta::SmfMeta, SmfError> {
        // Get the recording by name
        match (&self).get_recording_from_name(name) {
            Some(recording) => self.get_meta_from_recording(&recording),
            None => Err(SmfError::NotFound("Recording is None".to_string())),
        }
    }

    /// Get the samples from a [`SmfRecording`]
    ///
    /// # Arguments
    /// * `recording` - A [`SmfRecording`] object
    ///
    /// # Returns
    /// A vector of [`DataSample`] objects
    ///
    /// [`SmfRecording`]: crate::recording::SmfRecording
    /// [`DataSample`]: crate::utils::data::DataSample
    pub fn get_samples_from_recording(
        &self,
        recording: &recording::SmfRecording,
    ) -> Result<Vec<DataSample>, SmfError> {
        self.get_samples_from_name(&recording.get_name())
    }

    /// Get the samples from a [`SmfRecording`] by name
    ///
    /// # Arguments
    /// * `name` - A string slice that holds the name of the [`SmfRecording`]
    ///
    /// # Returns
    /// A vector of [`DataSample`] objects
    ///
    /// [`SmfRecording`]: crate::recording::SmfRecording
    /// [`DataSample`]: crate::utils::data::DataSample
    pub fn get_samples_from_name(&self, name: &str) -> Result<Vec<DataSample>, SmfError> {
        let meta: meta::SmfMeta = self.get_meta_from_name(name)?;
        let data: Vec<u8> = match &self.archive_path {
            Some(path) => {
                let (data, _) =
                    utils::file::get_bytes_in_archive_from_name_ext(path, name, "sigmf-data")?;
                data
            }
            None => {
                let current_dir = env::current_dir().map_err(|e| {
                    eprintln!("Error getting current directory: {}", e);
                    SmfError::NotFound("Path is None".to_string())
                })?;
                let file_path = current_dir.join(format!("{}.sigmf-data", name));
                let mut file = File::open(&file_path)?;
                let mut data = Vec::new();
                file.read_to_end(&mut data)?; // Read entire file into buffer
                data
            }
        };

        let data_type = meta.get_datatype();
        match data_type.get_domain() {
            global::SmfDataTypeDomain::Complex => match data_type.get_byte() {
                Some(byte_type) => Ok(utils::data::handle_complex_bytes(&data, byte_type)),
                None => {
                    if let (Some(dtype), Some(endian)) =
                        (data_type.get_dtype(), data_type.get_endian())
                    {
                        Ok(utils::data::handle_complex_data(&data, dtype, endian))
                    } else {
                        Err(SmfError::InvalidDataType("Invalid data type".to_string()))
                    }
                }
            },
            global::SmfDataTypeDomain::Real => match data_type.get_byte() {
                Some(byte_type) => Ok(utils::data::handle_real_bytes(&data, byte_type)),
                None => {
                    if let (Some(dtype), Some(endian)) =
                        (data_type.get_dtype(), data_type.get_endian())
                    {
                        Ok(utils::data::handle_real_data(&data, dtype, endian))
                    } else {
                        Err(SmfError::InvalidDataType("Invalid data type".to_string()))
                    }
                }
            },
        }
    }
}

pub struct Builder {
    tar_builder: tar::Builder<File>,
    path: String,
}

impl Builder {
    /// Create a new [`Builder`] object using a path.
    /// If the file already exists, it will be removed and a new file will be created.
    ///
    /// # Arguments
    /// * `path` - A string slice that holds the path to the desired .sigmf file
    ///
    /// # Returns
    /// A new [`Builder`] object
    ///
    /// [`Builder`]: crate::Builder
    pub fn new(path: String) -> Self {
        // check if file exists
        if let Ok(_) = File::open(&path) {
            // remove file if it exists
            std::fs::remove_file(&path).unwrap();
        }
        Builder {
            tar_builder: tar::Builder::new(File::create(path.clone()).unwrap()),
            path,
        }
    }

    /// Create a new [`SigMF`] object from a data or meta file.
    ///
    /// # Arguments
    /// * `filepath` - A string slice that holds the path to the .sigmf file
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    pub fn from_recording(filepath: &String) -> Result<SigMF, Box<dyn std::error::Error>> {
        if filepath.contains(".sigmf-data") {
            let base_name_fp = filepath.replace(".sigmf-data", "");
            let base_name = Path::new(&base_name_fp)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            let meta_path = format!("{}.sigmf-meta", base_name_fp);
            let hash = hash_from_path(&meta_path)?;

            let recording = SmfRecording::new(&base_name, &hash);

            let mut sigmf = SigMF::default();
            sigmf.add_recording(recording);

            Ok(sigmf)
        } else if filepath.contains(".sigmf-meta") {
            let base_name_fp = filepath.replace(".sigmf-meta", "");
            let base_name = Path::new(&base_name_fp)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            let hash = hash_from_path(filepath)?;
            let recording = SmfRecording::new(&base_name, &hash);

            let mut sigmf = SigMF::default();
            sigmf.add_recording(recording);

            Ok(sigmf)
        } else {
            Err(format!("Invalid file path: {}", filepath).into())
        }
    }

    /// Add meta data to the .sigmf file.
    ///
    /// Serializes the meta data and appends it to the tar file.
    ///
    /// # Arguments
    /// * `filename` - A string slice that holds the name of the file
    /// * `meta` - A [`SmfMeta`] object
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    ///
    /// [`SmfMeta`]: crate::meta::SmfMeta
    pub fn add_meta(&mut self, filename: &str, meta: meta::SmfMeta) -> Result<(), SmfError> {
        let meta_bytes = serde_json::to_vec(&meta)?;
        let meta_header = utils::file::get_meta_header(filename, &meta_bytes)?;

        self.tar_builder.append(&meta_header, &meta_bytes[..])?;
        Ok(())
    }

    /// Add data to the .sigmf file.
    /// Appends the data to the tar file.
    ///
    /// # Arguments
    /// * `filename` - A string slice that holds the name of the file
    /// * `data` - A slice of bytes that holds the data
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    pub fn add_data(&mut self, filename: &str, data: &[u8]) -> Result<(), SmfError> {
        let data_header = utils::file::get_data_header(filename, &data)?;
        self.tar_builder.append(&data_header, data)?;
        Ok(())
    }

    /// Add meta data and data to the .sigmf file.
    /// Serializes the meta data and appends it to the tar file.
    /// Appends the data to the tar file.
    ///
    /// # Arguments
    /// * `filename` - A string slice that holds the name of the file
    /// * `meta` - A [`SmfMeta`] object
    /// * `data` - A slice of bytes that holds the data
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    ///
    /// [`SmfMeta`]: crate::meta::SmfMeta
    pub fn add(
        &mut self,
        filename: &str,
        meta: meta::SmfMeta,
        data: &[u8],
    ) -> Result<(), SmfError> {
        self.add_meta(filename, meta)?;
        self.add_data(filename, data)
    }

    /// Add a collection to the .sigmf file.
    /// Serializes the collection and appends it to the tar file.
    /// Appends the data to the tar file.
    ///
    /// # Arguments
    /// * `filename` - A string slice that holds the name of the file
    /// * `collection` - A [`SmfCollection`] object
    ///     
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    ///
    /// [`SmfCollection`]: crate::collection::SmfCollection
    pub fn add_collection(
        &mut self,
        filename: &str,
        collection: collection::SmfCollection,
    ) -> Result<(), SmfError> {
        let collection_bytes = serde_json::to_vec(&collection)?;
        let collection_header = utils::file::get_collection_header(filename, &collection_bytes)?;

        self.tar_builder
            .append(&collection_header, &collection_bytes[..])?;
        Ok(())
    }

    /// Build the final [`SigMF`] object    
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful,
    /// or a SmfError if an error occurred
    ///
    /// [`SigMF`]: crate::SigMF
    pub fn build(&mut self) -> Result<SigMF, SmfError> {
        self.tar_builder.finish()?;
        Builder::load(&self.path)
    }

    /// Load a [`SigMF`] object from a .sigmf file
    ///
    /// # Arguments
    /// * `path` - A string slice that holds the path to the .sigmf file
    ///
    /// # Returns
    /// A Result object with the SigMF object if successful, or a SmfError if an error occurred
    ///
    /// [`SigMF`]: crate::SigMF
    pub fn load(path: &str) -> Result<SigMF, SmfError> {
        assert!(path.ends_with(".sigmf"), "File must have .sigmf extension");

        // Create the SigMF object
        let path_string = path.to_string();
        let mut sigmf = SigMF::default();
        sigmf.set_path(&path_string);

        // Open the tar file
        let file = File::open(&path_string)?;
        let mut archive = Archive::new(file);

        // Process each file in the archive
        for file in archive.entries()? {
            let mut file = file?;
            let file_path = file.path()?.to_path_buf();
            let file_path_str = format!("{}", file_path.display());

            if let (Some(file_name), Some(file_ext)) = (
                file_path.file_stem().and_then(|s| s.to_str()),
                file_path.extension().and_then(|s| s.to_str()),
            ) {
                match file_ext {
                    "sigmf-meta" => {
                        utils::file::process_meta_file(&mut file, &file_name, &mut sigmf)
                    }
                    "sigmf-collection" => {
                        utils::file::process_collection_file(&mut file, &mut sigmf)
                    }
                    _ => continue,
                }
            } else {
                utils::debug::print_warning(&file_path_str, "Skipping file with invalid extension");
                continue;
            }
        }

        // Return the SigMF object
        Ok(sigmf)
    }
}

#[derive(Debug)]
pub enum SmfError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    InvalidDataType(String),
    NotFound(String),
    HashMismatch(String),
    LockError(String),
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
            SmfError::HashMismatch(err) => write!(f, "Hash mismatch: {}", err),
            SmfError::LockError(err) => write!(f, "Lock error: {}", err),
        }
    }
}

impl Error for SmfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SmfError::Io(err) => Some(err),
            SmfError::Serde(err) => Some(err),
            SmfError::InvalidDataType(_err) => None,
            SmfError::NotFound(_err) => None,
            SmfError::HashMismatch(_err) => None,
            SmfError::LockError(_err) => None,
        }
    }
}
