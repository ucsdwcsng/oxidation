//! Read, write, and validate [SigMF] (Signal Metadata Format) recordings.
//!
//! SigMF describes a recording of digital samples with a JSON metadata file
//! (`.sigmf-meta`) stored next to a binary dataset (`.sigmf-data`). Recordings
//! may be grouped by a `.sigmf-collection` file and bundled into a single
//! `.sigmf` tar archive. This crate implements the SigMF 1.2 specification:
//!
//! - **Metadata** — strongly typed [`global`], [`capture`], and [`annotation`]
//!   objects that round-trip unknown and extension fields losslessly
//!   ([`meta::Metadata`]).
//! - **Datasets** — every SigMF [datatype](datatype::Datatype) (`cf32_le`,
//!   `ri16_be`, `ru8`, ...) with multi-channel, non-conforming (NCD), and
//!   streaming access ([`recording_file::Recording`],
//!   [`recording_file::RecordingWriter`], [`utils::data`]).
//! - **Archives** — create and read `.sigmf` tar archives ([`ArchiveWriter`],
//!   [`Archive`]) with SHA-512 integrity checks.
//! - **Collections** — `.sigmf-collection` files ([`collection::Collection`]).
//! - **Validation** — structural and semantic checks with machine-readable
//!   issues ([`validation`]).
//! - **Extensions** — typed, namespaced metadata extensions
//!   ([`extensions::SigmfExtension`], with an optional derive macro).
//!
//! # Quick start
//!
//! Write a recording to disk and stream it back:
//!
//! ```
//! use sigmf::datatype::Datatype;
//! use sigmf::global::Global;
//! use sigmf::meta::Metadata;
//! use sigmf::num_complex::Complex;
//! use sigmf::recording_file::{Recording, RecordingWriter};
//! use sigmf::utils::data::SampleValue;
//! use std::str::FromStr;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let dir = std::env::temp_dir().join(format!("sigmf-crate-doc-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! let mut metadata = Metadata::new(
//!     Global::new("1.2.6", Datatype::from_str("cf32_le")?),
//!     Vec::new(),
//!     Vec::new(),
//! );
//! metadata.set_sample_rate(1e6);
//! metadata.set_description("A short tone".to_string());
//!
//! // Writes `tone.sigmf-data`, then `tone.sigmf-meta` with `core:sha512` filled in.
//! let mut writer = RecordingWriter::new(dir.join("tone.sigmf-meta"), metadata)?;
//! let samples: Vec<SampleValue> = (0..1024)
//!     .map(|n| {
//!         let phase = n as f32 * 0.1;
//!         SampleValue::ComplexF32(Complex::new(phase.cos(), phase.sin()))
//!     })
//!     .collect();
//! writer.write_frames(&samples)?;
//! writer.finish()?;
//!
//! let mut recording = Recording::open(dir.join("tone.sigmf-meta"))?;
//! recording.verify_dataset_digest()?;
//! assert_eq!(recording.frame_count()?, 1024);
//! let start = recording.first_sample_index();
//! for block in recording.frame_blocks(start, 1024, 256)? {
//!     let block = block?;
//!     assert_eq!(block.len(), 256);
//! }
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok(())
//! # }
//! ```
//!
//! Bundle recordings into a `.sigmf` archive and read them back:
//!
//! ```
//! use sigmf::datatype::Datatype;
//! use sigmf::global::Global;
//! use sigmf::meta::Metadata;
//! use sigmf::{Archive, ArchiveWriter};
//! use std::str::FromStr;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let dir = std::env::temp_dir().join(format!("sigmf-crate-doc-archive-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! let path = dir.join("capture.sigmf").to_string_lossy().into_owned();
//! let metadata = Metadata::new(
//!     Global::new("1.2.6", Datatype::from_str("ri8")?),
//!     Vec::new(),
//!     Vec::new(),
//! );
//!
//! let mut writer = ArchiveWriter::new(path.clone())?;
//! writer.add("burst", metadata, &[1u8, 2, 3, 4])?;
//! writer.build()?;
//!
//! let archive = Archive::load(&path)?;
//! assert_eq!(archive.get_names(), ["burst"]);
//! let samples = archive.get_samples_from_name("burst")?;
//! assert_eq!(samples.len(), 4); // `ri8` is real 8-bit: one byte per sample
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Cargo features
//!
//! | Feature  | Description |
//! |----------|-------------|
//! | `derive` | Re-exports [`SigmfExtension`](macro@SigmfExtension), a derive macro for [`extensions::SigmfExtension`]. |
//! | `schema` | JSON Schema generation and validation for extensions ([`extensions::extension_schema`], [`extensions::validate_json_schema`]). |
//!
//! No features are enabled by default.
//!
//! [SigMF]: https://sigmf.org
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

use core::fmt;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use tar::Archive as TarArchive;
use utils::data::SampleValue;

pub mod annotation;
pub mod capture;
pub mod collection;
pub mod datatype;
pub mod extensions;
pub mod global;
mod json;
pub mod meta;
mod persistence;
pub mod recording;
pub mod recording_file;
pub mod utils;
pub mod validation;

/// Derives [`extensions::SigmfExtension`] for a struct of namespaced fields.
///
/// Every field needs an explicit `#[serde(rename = "namespace:field")]` that
/// matches the container's `#[sigmf(namespace = "...")]`. The required
/// container attributes are `namespace`, `version`, and `scope` (one of
/// `global`, `capture`, `annotation`, or `collection`). If the crate is
/// renamed in `Cargo.toml`, pass its path with `crate = "..."`.
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use sigmf::SigmfExtension;
///
/// #[derive(Serialize, Deserialize, SigmfExtension)]
/// #[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
/// struct LabGain {
///     #[serde(rename = "lab:gain_db")]
///     gain_db: f64,
/// }
///
/// use sigmf::extensions::SigmfExtension as _;
/// assert_eq!(LabGain::FIELDS, ["lab:gain_db"]);
/// ```
#[cfg(feature = "derive")]
pub use sigmf_derive::SigmfExtension;

/// Re-export of [`num_complex`], whose [`Complex`](num_complex::Complex) type
/// appears in [`utils::data::SampleValue`].
pub use num_complex;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// JSON number type used for floating-point SigMF fields such as
/// `core:sample_rate` and `core:frequency`.
pub type Number = f64;
/// JSON integer type used for unsigned SigMF fields such as
/// `core:sample_start` and `core:sample_count`.
pub type Integer = u64;

const MAX_META_SIZE: usize = 1_000_000;

/// An opened `.sigmf` archive: a tar file of recordings and an optional collection.
///
/// Metadata members are indexed and parsed when the archive is loaded; dataset
/// members are read lazily on request. Each [`recording::RecordingRef`] carries
/// the SHA-512 digest of its metadata member, which is re-checked whenever the
/// metadata is read back from the archive.
///
/// An archive created with [`Default`] has no [`archive_path`](Self::archive_path);
/// in that case recordings are resolved as `<name>.sigmf-meta` and
/// `<name>.sigmf-data` files in the current working directory.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> Result<(), sigmf::SigmfError> {
/// let archive = sigmf::Archive::load("recordings.sigmf")?;
/// for recording in archive.get_recordings() {
///     let metadata = archive.get_meta_from_recording(&recording)?;
///     println!("{}: {}", recording.get_name(), metadata.global().datatype);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct Archive {
    /// The archive's `.sigmf-collection` object, if it has one.
    pub collection: Option<collection::Collection>,
    /// Name (file stem) of the collection member, e.g. `"collection"` for
    /// `collection.sigmf-collection`.
    pub collection_name: Option<String>,
    /// References to the recordings in the archive, in member order.
    pub recordings: Vec<recording::RecordingRef>,
    /// Filesystem path of the `.sigmf` tar file backing this archive.
    pub archive_path: Option<String>,
    metadata_by_name: HashMap<String, meta::Metadata>,
    member_index: HashMap<std::path::PathBuf, ArchiveMemberLocation>,
}

#[derive(Debug, Clone, Copy)]
struct ArchiveMemberLocation {
    offset: u64,
    size: u64,
}

impl Archive {
    // ---------------------------------------- Constructors ---------------------------------------- //

    /// Creates an empty archive handle pointing at `path` without reading it.
    ///
    /// Use [`Archive::load`] to open and index an existing archive.
    pub fn new(path: String) -> Self {
        Archive {
            collection: None,
            collection_name: None,
            recordings: vec![],
            archive_path: Some(path),
            metadata_by_name: HashMap::new(),
            member_index: HashMap::new(),
        }
    }

    /// Opens and indexes a `.sigmf` tar archive.
    ///
    /// All `.sigmf-meta` members are parsed and at most one top-level
    /// `.sigmf-collection` member is accepted. Equivalent to
    /// [`ArchiveWriter::load`].
    ///
    /// # Errors
    ///
    /// Returns an error if `path` does not end in `.sigmf`, cannot be read, is
    /// not a valid tar file, contains unsafe or duplicate member paths,
    /// non-regular members, invalid metadata, oversized (> 1 MB) metadata, or
    /// no recordings.
    pub fn load(path: &str) -> Result<Self, SigmfError> {
        ArchiveWriter::load(path)
    }

    // ------------------------------------------- Methods ------------------------------------------ //

    /// Writes a new `.sigmf` archive at `save_path` containing only this
    /// archive's current [`recordings`](Self::recordings).
    ///
    /// Members are copied from the source archive at
    /// [`archive_path`](Self::archive_path). Metadata and dataset members of
    /// recordings that were removed (for example by
    /// [`retain_recordings_with_minimum_frames`](Self::retain_recordings_with_minimum_frames))
    /// are dropped, other members are preserved, and the collection is replaced
    /// when [`collection`](Self::collection) is set. The destination must not
    /// already exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the source path is unset, the destination exists,
    /// a referenced member is missing, or a metadata digest no longer matches
    /// its [`recording::RecordingRef`].
    pub fn save(&self, save_path: &str) -> Result<(), SigmfError> {
        let archive_path = self
            .archive_path
            .as_deref()
            .ok_or_else(|| SigmfError::NotFound("archive source path is not set".into()))?;
        let mut metadata_members = HashMap::new();
        let mut dataset_members = HashSet::new();
        let mut required_datasets = HashSet::new();
        for recording in &self.recordings {
            let name = recording.get_name();
            validate_recording_name(name)?;
            let metadata_path = std::path::PathBuf::from(format!("{name}.sigmf-meta"));
            if metadata_members
                .insert(metadata_path, recording.get_hash().to_string())
                .is_some()
            {
                return Err(SigmfError::InvalidMetadata(format!(
                    "duplicate recording name {name:?}"
                )));
            }
            let metadata = self.get_meta_from_recording(recording)?;
            if let Some(dataset_member) = archive_dataset_member(name, &metadata)? {
                required_datasets.insert(dataset_member.clone());
                dataset_members.insert(dataset_member);
            }
        }

        let mut writer = ArchiveWriter::new(save_path.to_string())?;
        let mut source = TarArchive::new(File::open(archive_path)?);
        let mut seen_members = HashSet::new();
        let mut seen_metadata = HashSet::new();
        let mut seen_datasets = HashSet::new();
        for entry in source.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            validate_archive_member_path(&path)?;
            if !seen_members.insert(path.clone()) {
                return Err(SigmfError::InvalidMetadata(format!(
                    "archive contains a duplicate member path: {}",
                    path.display()
                )));
            }
            let entry_type = entry.header().entry_type();
            if entry_type.is_dir() {
                let header = entry.header().clone();
                writer.builder_mut()?.append(&header, &mut entry)?;
                continue;
            }
            if !entry_type.is_file() {
                return Err(SigmfError::UnsupportedLayout(format!(
                    "archive member {} is not a regular file",
                    path.display()
                )));
            }
            let header = entry.header().clone();
            if let Some(expected_hash) = metadata_members.get(&path) {
                if entry.size() > MAX_META_SIZE as u64 {
                    return Err(SigmfError::ResourceLimit(format!(
                        "metadata member {} exceeds the {} byte limit",
                        path.display(),
                        MAX_META_SIZE
                    )));
                }
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes)?;
                let actual_hash = utils::hash::hash_from_slice(&bytes)?;
                if &actual_hash != expected_hash {
                    return Err(SigmfError::HashMismatch(format!(
                        "metadata digest mismatch for {}",
                        path.display()
                    )));
                }
                writer.builder_mut()?.append(&header, &bytes[..])?;
                seen_metadata.insert(path);
            } else if dataset_members.contains(&path) {
                writer.builder_mut()?.append(&header, &mut entry)?;
                seen_datasets.insert(path);
            } else if path.extension().and_then(|extension| extension.to_str())
                == Some("sigmf-collection")
            {
                if self.collection.is_none() {
                    writer.builder_mut()?.append(&header, &mut entry)?;
                }
            } else if matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("sigmf-meta" | "sigmf-data")
            ) {
                continue;
            } else {
                writer.builder_mut()?.append(&header, &mut entry)?;
            }
        }
        if seen_metadata.len() != metadata_members.len() {
            return Err(SigmfError::NotFound(
                "one or more referenced metadata members are missing".into(),
            ));
        }
        if seen_datasets != required_datasets {
            return Err(SigmfError::NotFound(
                "one or more referenced dataset members are missing".into(),
            ));
        }
        if let Some(collection) = &self.collection {
            let name = self.collection_name.as_deref().unwrap_or("collection");
            writer.add_collection(name, collection.clone())?;
        }
        writer.build()?;
        Ok(())
    }

    fn read_archive_member(
        &self,
        archive_path: &str,
        location: &ArchiveMemberLocation,
    ) -> Result<Vec<u8>, SigmfError> {
        let length = usize::try_from(location.size).map_err(|_| {
            SigmfError::ResourceLimit("archive member does not fit in memory".into())
        })?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|error| {
            SigmfError::ResourceLimit(format!("cannot allocate archive member buffer: {error}"))
        })?;
        bytes.resize(length, 0);
        let mut file = File::open(archive_path)?;
        file.seek(SeekFrom::Start(location.offset))?;
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    /// Removes recordings whose datasets hold fewer than `minimum_frames` frames.
    ///
    /// Frame counts are derived from member sizes, `core:num_channels`,
    /// `core:datatype`, NCD `core:header_bytes`, and `core:trailing_bytes`
    /// without reading sample data. Metadata-only recordings count as zero
    /// frames.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata cannot be read or a dataset layout is
    /// inconsistent with its size. On error the recordings are left unchanged.
    pub fn retain_recordings_with_minimum_frames(
        &mut self,
        minimum_frames: Integer,
    ) -> Result<(), SigmfError> {
        let mut retained = Vec::with_capacity(self.recordings.len());
        for recording in &self.recordings {
            let metadata = self.get_meta_from_recording(recording)?;
            let frame_count = if metadata.global().metadata_only == Some(true) {
                0
            } else {
                let data_size = match &self.archive_path {
                    Some(path) => {
                        let member = archive_dataset_member(recording.get_name(), &metadata)?
                            .ok_or(SigmfError::NoDataset)?;
                        match self.member_index.get(&member) {
                            Some(location) => location.size,
                            None => archive_member_size(path, &member)?,
                        }
                    }
                    None => std::fs::metadata(recording_data_path(recording.get_name())?)?.len(),
                };
                let trailer = metadata.global().trailing_bytes.unwrap_or(0);
                if trailer > data_size {
                    return Err(SigmfError::UnsupportedLayout(
                        "core:trailing_bytes exceeds the dataset file size".to_string(),
                    ));
                }
                let offset = metadata.global().offset.unwrap_or(0);
                let header_bytes = metadata
                    .captures()
                    .iter()
                    .filter(|capture| capture.sample_start >= offset)
                    .try_fold(0u64, |total, capture| {
                        total.checked_add(capture.header_bytes.unwrap_or(0))
                    })
                    .ok_or_else(|| {
                        SigmfError::UnsupportedLayout("capture header size overflows".into())
                    })?;
                let sample_bytes = data_size
                    .checked_sub(trailer)
                    .and_then(|bytes| bytes.checked_sub(header_bytes))
                    .ok_or_else(|| {
                        SigmfError::UnsupportedLayout(
                            "capture headers and trailing bytes exceed dataset size".into(),
                        )
                    })?;
                let channels = usize::try_from(metadata.global().num_channels.unwrap_or(1))
                    .map_err(|_| {
                        SigmfError::InvalidMetadata(
                            "core:num_channels does not fit usize".to_string(),
                        )
                    })?;
                let frames =
                    utils::data::frame_count(sample_bytes, &metadata.global().datatype, channels)?;
                if metadata.captures().iter().any(|capture| {
                    capture.sample_start >= offset
                        && capture.sample_start - offset > frames
                        && capture.header_bytes.unwrap_or(0) > 0
                }) {
                    return Err(SigmfError::UnsupportedLayout(
                        "capture header starts beyond the dataset sample range".into(),
                    ));
                }
                frames
            };
            if frame_count >= minimum_frames {
                retained.push(recording.clone());
            }
        }
        self.recordings = retained;
        Ok(())
    }

    // ------------------------------------------- Setters ------------------------------------------ //

    /// Replaces the list of recording references.
    pub fn set_recordings(&mut self, recordings: Vec<recording::RecordingRef>) {
        self.recordings = recordings;
    }

    /// Sets the collection, naming it `collection` if no name is set yet.
    pub fn set_collection(&mut self, collection: collection::Collection) {
        self.collection = Some(collection);
        if self.collection_name.is_none() {
            self.collection_name = Some("collection".into());
        }
    }

    /// Sets the collection along with its archive member name (file stem).
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `name` is not a safe,
    /// top-level member name.
    pub fn set_collection_member(
        &mut self,
        name: impl Into<String>,
        collection: collection::Collection,
    ) -> Result<(), SigmfError> {
        let name = name.into();
        validate_recording_name(&name)?;
        if name.contains('/') {
            return Err(SigmfError::InvalidMetadata(
                "collection metadata must be a top-level archive member".into(),
            ));
        }
        self.collection = Some(collection);
        self.collection_name = Some(name);
        Ok(())
    }

    /// Sets the path of the backing `.sigmf` file.
    pub fn set_path(&mut self, path: &str) {
        self.archive_path = Some(path.to_string());
    }

    /// Appends a recording reference.
    pub fn add_recording(&mut self, recording: recording::RecordingRef) {
        self.recordings.push(recording);
    }

    pub(crate) fn add_recording_with_metadata(
        &mut self,
        recording: recording::RecordingRef,
        metadata: meta::Metadata,
    ) {
        self.metadata_by_name
            .insert(recording.get_name().to_owned(), metadata);
        self.recordings.push(recording);
    }

    /// Sets the collection; alias for [`Archive::set_collection`].
    pub fn add_collection(&mut self, collection: collection::Collection) {
        self.set_collection(collection);
    }

    // ------------------------------------------- Getters ------------------------------------------ //

    /// Returns a copy of the recording references.
    pub fn get_recordings(&self) -> Vec<recording::RecordingRef> {
        self.recordings.clone()
    }

    /// Returns a copy of the collection, if any.
    pub fn get_collection(&self) -> Option<collection::Collection> {
        self.collection.clone()
    }

    /// Returns the path of the backing `.sigmf` file, if set.
    pub fn get_path(&self) -> Option<String> {
        self.archive_path.clone()
    }

    /// Returns the name of every recording, in order.
    ///
    /// A recording's name is its member path without the `.sigmf-meta`
    /// extension, e.g. `"dir/burst"` for `dir/burst.sigmf-meta`.
    pub fn get_names(&self) -> Vec<String> {
        self.recordings
            .par_iter()
            .map(|recording| recording.get_name().to_owned())
            .collect()
    }

    /// Finds a recording reference by name.
    pub fn get_recording_from_name(&self, name: &str) -> Option<recording::RecordingRef> {
        self.recordings
            .par_iter()
            .find_any(|recording| recording.get_name() == name)
            .cloned()
    }

    /// Returns the parsed metadata for `recording`.
    ///
    /// Metadata parsed at load time is reused when its digest still matches;
    /// otherwise the member is re-read and its SHA-512 digest is checked
    /// against [`RecordingRef::get_hash`](recording::RecordingRef::get_hash).
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::HashMismatch`] if the digest differs,
    /// [`SigmfError::NotFound`] if the member is missing, or a parse error.
    pub fn get_meta_from_recording(
        &self,
        recording: &recording::RecordingRef,
    ) -> Result<meta::Metadata, SigmfError> {
        if let Some(metadata) = self.metadata_by_name.get(recording.get_name())
            && metadata.original_digest()?.as_deref() == Some(recording.get_hash())
        {
            return Ok(metadata.clone());
        }
        match &self.archive_path {
            Some(path) => {
                let metadata_path =
                    std::path::PathBuf::from(format!("{}.sigmf-meta", recording.get_name()));
                let bytes = match self.member_index.get(&metadata_path) {
                    Some(location) => {
                        if location.size > MAX_META_SIZE as u64 {
                            return Err(SigmfError::ResourceLimit(format!(
                                "metadata member {} exceeds the {} byte limit",
                                metadata_path.display(),
                                MAX_META_SIZE
                            )));
                        }
                        self.read_archive_member(path, location)?
                    }
                    None => read_archive_member_by_path(
                        path,
                        &metadata_path,
                        Some(MAX_META_SIZE as u64),
                    )?,
                };
                let digest = utils::hash::hash_from_slice(&bytes)?;
                if digest != recording.get_hash() {
                    return Err(SigmfError::HashMismatch(format!(
                        "metadata digest mismatch for {:?}",
                        recording.get_name()
                    )));
                }
                meta::Metadata::from_slice(&bytes)
            }
            None => {
                let path = env::current_dir()?;
                let metadata_path = recording_metadata_path(&path, recording.get_name())?;
                meta::Metadata::from_path(metadata_path)
            }
        }
    }

    /// Returns the parsed metadata for the recording called `name`.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::NotFound`] if no recording has that name, or any
    /// error from [`Archive::get_meta_from_recording`].
    pub fn get_meta_from_name(&self, name: &str) -> Result<meta::Metadata, SigmfError> {
        // Get the recording by name
        match self.get_recording_from_name(name) {
            Some(recording) => self.get_meta_from_recording(&recording),
            None => Err(SigmfError::NotFound("Recording is None".to_string())),
        }
    }

    /// Reads and decodes every sample of `recording` into memory.
    ///
    /// See [`Archive::get_samples_from_name`].
    pub fn get_samples_from_recording(
        &self,
        recording: &recording::RecordingRef,
    ) -> Result<Vec<SampleValue>, SigmfError> {
        self.get_samples_from_name(recording.get_name())
    }

    /// Reads and decodes every sample of the recording called `name` into memory.
    ///
    /// Samples are returned frame by frame, interleaved across
    /// `core:num_channels`. The whole dataset member is decoded as plain
    /// samples, so use [`recording_file::Recording`] for NCD datasets or for
    /// streaming large recordings.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::NoDataset`] for metadata-only recordings, or an
    /// error if the dataset cannot be read or does not hold whole frames.
    pub fn get_samples_from_name(&self, name: &str) -> Result<Vec<SampleValue>, SigmfError> {
        let meta: meta::Metadata = self.get_meta_from_name(name)?;
        let data: Vec<u8> = match &self.archive_path {
            Some(path) => {
                let member = archive_dataset_member(name, &meta)?.ok_or(SigmfError::NoDataset)?;
                match self.member_index.get(&member) {
                    Some(location) => self.read_archive_member(path, location)?,
                    None => read_archive_member_by_path(path, &member, None)?,
                }
            }
            None => {
                let current_dir = env::current_dir()?;
                let file_path = recording_data_path_from(&current_dir, name)?;
                let mut file = File::open(&file_path)?;
                let mut data = Vec::new();
                file.read_to_end(&mut data)?; // Read entire file into buffer
                data
            }
        };

        let data_type = meta.get_datatype();
        let channel_count = meta
            .get_num_channels()
            .unwrap_or(1)
            .try_into()
            .map_err(|_| {
                SigmfError::InvalidMetadata("core:num_channels does not fit usize".to_string())
            })?;
        utils::data::decode(&data, &data_type, channel_count)
    }
}

/// Builds a new `.sigmf` archive.
///
/// Members are streamed into a hidden temporary file next to the destination,
/// which is only published at the destination path by [`ArchiveWriter::build`].
/// An existing destination is never overwritten.
///
/// # Examples
///
/// ```
/// use sigmf::{datatype::Datatype, global::Global, meta::Metadata, ArchiveWriter};
/// use std::str::FromStr;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let dir = std::env::temp_dir().join(format!("sigmf-archive-writer-doc-{}", std::process::id()));
/// # std::fs::create_dir_all(&dir)?;
/// # let path = dir.join("example.sigmf").to_string_lossy().into_owned();
/// let metadata = Metadata::new(Global::new("1.2.6", Datatype::from_str("ru8")?), vec![], vec![]);
/// let mut writer = ArchiveWriter::new(path)?;
/// writer.add("ramp", metadata, &[0, 1, 2, 3])?;
/// let archive = writer.build()?;
/// assert_eq!(archive.get_names(), ["ramp"]);
/// # std::fs::remove_dir_all(&dir)?;
/// # Ok(())
/// # }
/// ```
pub struct ArchiveWriter {
    tar_builder: Option<tar::Builder<File>>,
    path: String,
    temporary_path: std::path::PathBuf,
    metadata_hashes: HashMap<String, String>,
    dataset_names: HashSet<String>,
    finished: bool,
}

impl ArchiveWriter {
    /// Starts a new archive that will be published at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if `path` does not end in `.sigmf`, already exists, or
    /// a temporary file cannot be created in its directory.
    pub fn new(path: String) -> Result<Self, SigmfError> {
        if !path.ends_with(".sigmf") {
            return Err(SigmfError::InvalidMetadata(
                "archive path must end with .sigmf".to_string(),
            ));
        }
        let destination = std::path::Path::new(&path);
        if destination.try_exists()? {
            return Err(SigmfError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "archive destination already exists",
            )));
        }
        let (temporary_path, file) = persistence::create_sibling_temp(destination)?;
        Ok(ArchiveWriter {
            tar_builder: Some(tar::Builder::new(file)),
            path,
            temporary_path,
            metadata_hashes: HashMap::new(),
            dataset_names: HashSet::new(),
            finished: false,
        })
    }

    fn builder_mut(&mut self) -> Result<&mut tar::Builder<File>, SigmfError> {
        self.tar_builder.as_mut().ok_or_else(|| {
            SigmfError::InvalidMetadata("archive writer has already been finalized".to_string())
        })
    }

    /// Appends `<filename>.sigmf-meta` without a dataset.
    ///
    /// Use this for metadata-only recordings, or pair it with
    /// [`ArchiveWriter::add_data`]. Prefer [`ArchiveWriter::add`], which also
    /// fills in `core:sha512`.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `filename` is unsafe or
    /// already used, or if the metadata fails
    /// [`ValidationPolicy::Interoperability`](validation::ValidationPolicy::Interoperability).
    pub fn add_meta(&mut self, filename: &str, meta: meta::Metadata) -> Result<(), SigmfError> {
        validate_recording_name(filename)?;
        if self.metadata_hashes.contains_key(filename) {
            return Err(SigmfError::InvalidMetadata(format!(
                "duplicate metadata member for recording {filename:?}"
            )));
        }
        let report = meta.validate(validation::ValidationPolicy::Interoperability);
        if report.status == validation::ValidationStatus::Failed {
            return Err(SigmfError::InvalidMetadata(
                "metadata failed validation and was not added to the archive".into(),
            ));
        }
        let meta_bytes = serde_json::to_vec(&meta)?;
        let meta_header = utils::file::get_meta_header(filename, &meta_bytes)?;

        self.builder_mut()?.append(&meta_header, &meta_bytes[..])?;
        self.metadata_hashes.insert(
            filename.to_owned(),
            utils::hash::hash_from_slice(&meta_bytes)?,
        );
        Ok(())
    }

    /// Appends raw dataset bytes as `<filename>.sigmf-data`.
    ///
    /// The bytes are written as-is; no digest is computed or checked.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `filename` is unsafe or the
    /// dataset member already exists.
    pub fn add_data(&mut self, filename: &str, data: &[u8]) -> Result<(), SigmfError> {
        validate_recording_name(filename)?;
        let member = std::path::PathBuf::from(format!("{filename}.sigmf-data"));
        self.add_dataset_member(&member, data)
    }

    fn add_dataset_member(
        &mut self,
        member: &std::path::Path,
        data: &[u8],
    ) -> Result<(), SigmfError> {
        validate_archive_member_path(member)?;
        let member_name = member.to_str().ok_or_else(|| {
            SigmfError::InvalidMetadata("archive member path must be valid UTF-8".into())
        })?;
        if self.dataset_names.contains(member_name) {
            return Err(SigmfError::InvalidMetadata(format!(
                "duplicate dataset member {member_name:?}"
            )));
        }
        let data_header = utils::file::get_data_header_for_member(member, data)?;
        self.builder_mut()?.append(&data_header, data)?;
        self.dataset_names.insert(member_name.to_owned());
        Ok(())
    }

    /// Appends a recording: its metadata and its encoded dataset bytes.
    ///
    /// `core:sha512` is set to the SHA-512 digest of `data`. The dataset member
    /// is `<filename>.sigmf-data`, or the file named by `core:dataset` (in the
    /// same archive directory as the metadata) when that field is present.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::HashMismatch`] if the metadata already declares a
    /// different `core:sha512`, [`SigmfError::UnsupportedLayout`] for
    /// metadata-only recordings, or any error from
    /// [`ArchiveWriter::add_meta`].
    pub fn add(
        &mut self,
        filename: &str,
        mut meta: meta::Metadata,
        data: &[u8],
    ) -> Result<(), SigmfError> {
        if meta.global().metadata_only == Some(true) {
            return Err(SigmfError::UnsupportedLayout(
                "metadata-only recordings cannot be added with dataset bytes".into(),
            ));
        }
        let dataset_member =
            archive_dataset_member(filename, &meta)?.ok_or(SigmfError::NoDataset)?;
        let digest = utils::hash::hash_from_slice(data)?;
        if meta
            .global()
            .sha512
            .as_deref()
            .is_some_and(|existing| !existing.eq_ignore_ascii_case(&digest))
        {
            return Err(SigmfError::HashMismatch(
                "provided dataset bytes do not match core:sha512".into(),
            ));
        }
        meta.set_sha512(digest);
        self.add_meta(filename, meta)?;
        self.add_dataset_member(&dataset_member, data)
    }

    /// Appends a top-level `<filename>.sigmf-collection` member.
    ///
    /// Stream hashes in `core:streams` are updated to the digests of metadata
    /// members already added to this writer, so add recordings first.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `filename` is not a safe
    /// top-level name or the collection fails validation.
    pub fn add_collection(
        &mut self,
        filename: &str,
        mut collection: collection::Collection,
    ) -> Result<(), SigmfError> {
        validate_recording_name(filename)?;
        if filename.contains('/') {
            return Err(SigmfError::InvalidMetadata(
                "collection metadata must be a top-level archive member".into(),
            ));
        }
        if collection
            .validate(validation::ValidationPolicy::Interoperability)
            .status
            == validation::ValidationStatus::Failed
        {
            return Err(SigmfError::InvalidMetadata(
                "collection failed validation and was not added to the archive".into(),
            ));
        }
        for recording in &mut collection.collection.streams {
            if let Some(hash) = self.metadata_hashes.get(recording.get_name()) {
                recording.set_hash(hash.clone());
            }
        }
        let collection_bytes = serde_json::to_vec(&collection)?;
        let collection_header = utils::file::get_collection_header(filename, &collection_bytes)?;

        self.builder_mut()?
            .append(&collection_header, &collection_bytes[..])?;
        Ok(())
    }

    /// Finalizes the tar file, publishes it at the destination path, and
    /// returns the loaded [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if the writer was already built, the written archive
    /// does not load (for example, it has no recordings), or the destination
    /// was created by someone else in the meantime.
    pub fn build(&mut self) -> Result<Archive, SigmfError> {
        if self.finished {
            return Err(SigmfError::InvalidMetadata(
                "archive writer has already been finalized".to_string(),
            ));
        }
        let mut builder = self.tar_builder.take().ok_or_else(|| {
            SigmfError::InvalidMetadata("archive writer has already been finalized".to_string())
        })?;
        builder.finish()?;
        builder.get_ref().sync_all()?;
        drop(builder);
        self.finished = true;
        let mut archive = load_archive_file(&self.temporary_path)?;
        persistence::publish_new(&self.temporary_path, std::path::Path::new(&self.path))?;
        archive.set_path(&self.path);
        Ok(archive)
    }

    /// Opens and indexes a `.sigmf` tar archive; see [`Archive::load`].
    pub fn load(path: &str) -> Result<Archive, SigmfError> {
        if !path.ends_with(".sigmf") {
            return Err(SigmfError::InvalidMetadata(
                "archive path must end with .sigmf".to_string(),
            ));
        }
        load_archive_file(std::path::Path::new(path))
    }
}

fn load_archive_file(path: &std::path::Path) -> Result<Archive, SigmfError> {
    let path_string = path.to_string_lossy().into_owned();
    let mut sigmf = Archive::default();
    sigmf.set_path(&path_string);

    let file = File::open(path)?;
    let mut archive = TarArchive::new(file);
    let mut member_paths = HashSet::new();
    let mut regular_member_paths = HashSet::new();
    let mut collection_count = 0usize;

    for file in archive.entries()? {
        let mut file = file?;
        let file_path = file.path()?.to_path_buf();
        validate_archive_member_path(&file_path)?;
        if !member_paths.insert(file_path.clone()) {
            return Err(SigmfError::InvalidMetadata(format!(
                "archive contains a duplicate member path: {}",
                file_path.display()
            )));
        }
        let entry_type = file.header().entry_type();
        if entry_type.is_dir() {
            continue;
        }
        if !entry_type.is_file() {
            return Err(SigmfError::UnsupportedLayout(format!(
                "archive member {} is not a regular file",
                file_path.display()
            )));
        }
        sigmf.member_index.insert(
            file_path.clone(),
            ArchiveMemberLocation {
                offset: file.raw_file_position(),
                size: file.size(),
            },
        );
        regular_member_paths.insert(file_path.clone());

        match file_path
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("sigmf-meta") => {
                let recording_name = archive_member_stem(&file_path)?;
                utils::file::process_meta_file(&mut file, &recording_name, &mut sigmf)?;
            }
            Some("sigmf-collection") => {
                if file_path.components().count() != 1 {
                    return Err(SigmfError::InvalidMetadata(
                        "collection metadata must be a top-level archive member".into(),
                    ));
                }
                collection_count += 1;
                if collection_count > 1 {
                    return Err(SigmfError::InvalidMetadata(
                        "archive contains more than one collection member".into(),
                    ));
                }
                let collection_name = archive_member_stem(&file_path)?;
                utils::file::process_collection_file(&mut file, &collection_name, &mut sigmf)?;
            }
            _ => continue,
        }
    }

    if sigmf.recordings.is_empty() {
        return Err(SigmfError::InvalidMetadata(
            "archive must contain at least one recording".into(),
        ));
    }
    for recording in &sigmf.recordings {
        let metadata = sigmf
            .metadata_by_name
            .get(recording.get_name())
            .ok_or_else(|| {
                SigmfError::InvalidMetadata(format!(
                    "metadata cache is missing recording {:?}",
                    recording.get_name()
                ))
            })?;
        if let Some(dataset_member) = archive_dataset_member(recording.get_name(), metadata)?
            && !regular_member_paths.contains(&dataset_member)
        {
            return Err(SigmfError::NotFound(format!(
                "dataset member {} is missing",
                dataset_member.display()
            )));
        }
    }
    Ok(sigmf)
}

fn archive_member_size(
    archive_path: &str,
    member_path: &std::path::Path,
) -> Result<u64, SigmfError> {
    let mut archive = TarArchive::new(File::open(archive_path)?);
    let mut found = None;
    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?.to_path_buf();
        validate_archive_member_path(&path)?;
        if path == member_path {
            if found.is_some() {
                return Err(SigmfError::InvalidMetadata(format!(
                    "archive contains a duplicate member path: {}",
                    member_path.display()
                )));
            }
            if !entry.header().entry_type().is_file() {
                return Err(SigmfError::UnsupportedLayout(format!(
                    "archive member {} is not a regular file",
                    member_path.display()
                )));
            }
            found = Some(entry.size());
        }
    }
    found.ok_or_else(|| {
        SigmfError::NotFound(format!(
            "archive member {} is missing",
            member_path.display()
        ))
    })
}

fn read_archive_member_by_path(
    archive_path: &str,
    member_path: &std::path::Path,
    maximum_size: Option<u64>,
) -> Result<Vec<u8>, SigmfError> {
    let mut archive = TarArchive::new(File::open(archive_path)?);
    let mut found = None;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        validate_archive_member_path(&path)?;
        if path == member_path {
            if found.is_some() {
                return Err(SigmfError::InvalidMetadata(format!(
                    "archive contains a duplicate member path: {}",
                    member_path.display()
                )));
            }
            if !entry.header().entry_type().is_file() {
                return Err(SigmfError::UnsupportedLayout(format!(
                    "archive member {} is not a regular file",
                    member_path.display()
                )));
            }
            if let Some(maximum) = maximum_size
                && entry.size() > maximum
            {
                return Err(SigmfError::ResourceLimit(format!(
                    "archive member {} exceeds the {maximum} byte limit",
                    member_path.display()
                )));
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            found = Some(bytes);
        }
    }
    found.ok_or_else(|| {
        SigmfError::NotFound(format!(
            "archive member {} is missing",
            member_path.display()
        ))
    })
}

impl Drop for ArchiveWriter {
    fn drop(&mut self) {
        drop(self.tar_builder.take());
        let _ = std::fs::remove_file(&self.temporary_path);
    }
}

fn archive_member_stem(path: &std::path::Path) -> Result<String, SigmfError> {
    let path = path.to_str().ok_or_else(|| {
        SigmfError::InvalidMetadata("archive recording name must be valid UTF-8".into())
    })?;
    let (parent, filename) = path.rsplit_once('/').unwrap_or(("", path));
    let stem = filename
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or(filename, |(stem, _)| stem);
    if stem.is_empty() {
        return Err(SigmfError::InvalidMetadata(
            "archive recording member has no filename".into(),
        ));
    }
    let name = if parent.is_empty() {
        stem.to_owned()
    } else {
        format!("{parent}/{stem}")
    };
    validate_recording_name(&name)?;
    Ok(name)
}

fn validate_recording_name(name: &str) -> Result<(), SigmfError> {
    let path = std::path::Path::new(name);
    if name.is_empty()
        || name.contains('\\')
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(SigmfError::InvalidMetadata(format!(
            "recording name must be a safe relative member path: {name:?}"
        )));
    }
    Ok(())
}

fn validate_dataset_filename(name: &str) -> Result<(), SigmfError> {
    let path = std::path::Path::new(name);
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.bytes().any(|byte| {
            byte.is_ascii_control()
                || matches!(byte, b':' | b'*' | b'?' | b'"' | b'<' | b'>' | b'|')
        })
        || path.components().count() != 1
        || !matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(SigmfError::InvalidMetadata(format!(
            "dataset name must be a safe filename: {name:?}"
        )));
    }
    Ok(())
}

fn validate_archive_member_path(path: &std::path::Path) -> Result<(), SigmfError> {
    if path.is_absolute()
        || path.to_string_lossy().contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(SigmfError::InvalidMetadata(format!(
            "archive contains an unsafe member path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn archive_dataset_member(
    recording_name: &str,
    metadata: &meta::Metadata,
) -> Result<Option<std::path::PathBuf>, SigmfError> {
    let dataset_name = metadata.global().dataset.as_deref();
    if metadata.global().metadata_only == Some(true) && dataset_name.is_none() {
        return Ok(None);
    }
    let member = if let Some(dataset_name) = dataset_name {
        validate_dataset_filename(dataset_name)?;
        match recording_name.rsplit_once('/') {
            Some((parent, _)) => std::path::PathBuf::from(format!("{parent}/{dataset_name}")),
            None => std::path::PathBuf::from(dataset_name),
        }
    } else {
        std::path::PathBuf::from(format!("{recording_name}.sigmf-data"))
    };
    validate_archive_member_path(&member)?;
    Ok(Some(member))
}

fn recording_metadata_path(
    root: &std::path::Path,
    name: &str,
) -> Result<std::path::PathBuf, SigmfError> {
    validate_recording_name(name)?;
    let mut filename = std::ffi::OsString::from(name);
    filename.push(".sigmf-meta");
    Ok(root.join(filename))
}

fn recording_data_path(name: &str) -> Result<std::path::PathBuf, SigmfError> {
    let root = env::current_dir()?;
    recording_data_path_from(&root, name)
}

fn recording_data_path_from(
    root: &std::path::Path,
    name: &str,
) -> Result<std::path::PathBuf, SigmfError> {
    validate_recording_name(name)?;
    let mut filename = std::ffi::OsString::from(name);
    filename.push(".sigmf-data");
    Ok(root.join(filename))
}

/// Errors returned by this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum SigmfError {
    /// An underlying filesystem or I/O operation failed.
    Io(std::io::Error),
    /// JSON serialization or deserialization failed.
    Serde(serde_json::Error),
    /// The input is not valid JSON for SigMF (for example, duplicate keys).
    InvalidJson(String),
    /// A `core:datatype` string is malformed or unsupported.
    InvalidDataType(String),
    /// Metadata, a path, or an argument violates the SigMF specification or
    /// this crate's requirements.
    InvalidMetadata(String),
    /// The dataset layout (NCD headers, trailing bytes, metadata-only, ...)
    /// is not supported by the requested operation.
    UnsupportedLayout(String),
    /// A requested sample range is outside the dataset.
    InvalidRange(String),
    /// An input exceeds a size limit, or memory could not be reserved.
    ResourceLimit(String),
    /// The recording has no dataset (it is metadata-only).
    NoDataset,
    /// A recording, archive member, or file was not found.
    NotFound(String),
    /// A SHA-512 digest does not match the expected value.
    HashMismatch(String),
    /// A lock could not be acquired.
    LockError(String),
}

impl From<std::io::Error> for SigmfError {
    fn from(error: std::io::Error) -> Self {
        SigmfError::Io(error)
    }
}

impl From<serde_json::Error> for SigmfError {
    fn from(error: serde_json::Error) -> Self {
        SigmfError::Serde(error)
    }
}

impl std::fmt::Display for SigmfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SigmfError::Io(err) => write!(f, "IO error: {}", err),
            SigmfError::Serde(err) => write!(f, "Serde error: {}", err),
            SigmfError::InvalidJson(err) => write!(f, "Invalid JSON: {}", err),
            SigmfError::InvalidDataType(err) => write!(f, "Invalid data type: {}", err),
            SigmfError::InvalidMetadata(err) => write!(f, "Invalid metadata: {}", err),
            SigmfError::UnsupportedLayout(err) => write!(f, "Unsupported dataset layout: {}", err),
            SigmfError::InvalidRange(err) => write!(f, "Invalid sample range: {}", err),
            SigmfError::ResourceLimit(err) => write!(f, "Resource limit exceeded: {}", err),
            SigmfError::NoDataset => write!(f, "recording has no available dataset"),
            SigmfError::NotFound(err) => write!(f, "Not found: {}", err),
            SigmfError::HashMismatch(err) => write!(f, "Hash mismatch: {}", err),
            SigmfError::LockError(err) => write!(f, "Lock error: {}", err),
        }
    }
}

impl Error for SigmfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SigmfError::Io(err) => Some(err),
            SigmfError::Serde(err) => Some(err),
            SigmfError::InvalidJson(_err) => None,
            SigmfError::InvalidDataType(_err) => None,
            SigmfError::InvalidMetadata(_err) => None,
            SigmfError::UnsupportedLayout(_err) => None,
            SigmfError::InvalidRange(_err) => None,
            SigmfError::ResourceLimit(_err) => None,
            SigmfError::NoDataset => None,
            SigmfError::NotFound(_err) => None,
            SigmfError::HashMismatch(_err) => None,
            SigmfError::LockError(_err) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatype::Datatype;
    use std::str::FromStr;

    fn temp_archive_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sigmf-archive-writer-test-{}-{}.sigmf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn archive_writer_publishes_only_after_finish_without_overwriting() {
        let path = temp_archive_path();
        {
            let writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
            assert!(!path.exists());
            drop(writer);
        }
        assert!(!path.exists());

        let datatype = Datatype::from_str("ru8").unwrap();
        let metadata = meta::Metadata::new(
            global::Global::new("1.2.6", datatype),
            Vec::new(),
            Vec::new(),
        );
        let mut writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
        writer.add("sample", metadata, &[1, 2, 3]).unwrap();
        let archive = writer.build().unwrap();
        assert_eq!(archive.recordings.len(), 1);
        assert!(ArchiveWriter::new(path.to_string_lossy().into_owned()).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archive_writer_rejects_missing_dataset_before_publication() {
        let path = temp_archive_path();
        let metadata = meta::Metadata::new(
            global::Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
            Vec::new(),
            Vec::new(),
        );
        let bytes = metadata.to_vec().unwrap();
        let header = utils::file::get_meta_header("orphan", &bytes).unwrap();
        let mut writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
        let temporary_path = writer.temporary_path.clone();
        writer
            .builder_mut()
            .unwrap()
            .append(&header, &bytes[..])
            .unwrap();
        assert!(writer.build().is_err());
        drop(writer);
        assert!(!path.exists());
        assert!(!temporary_path.exists());
    }

    #[test]
    fn archive_loader_rejects_unsafe_and_duplicate_members() {
        for (member_names, expected_error) in [
            (vec!["../escape"], "unsafe"),
            (vec!["extra.bin", "extra.bin"], "duplicate"),
        ] {
            let path = temp_archive_path();
            let mut builder = tar::Builder::new(File::create(&path).unwrap());
            for name in member_names {
                let mut header = tar::Header::new_gnu();
                header.set_size(0);
                header.set_mode(0o644);
                if name == "../escape" {
                    let path = b"../escape";
                    header.as_mut_bytes()[..path.len()].copy_from_slice(path);
                    header.as_mut_bytes()[path.len()] = 0;
                } else {
                    header.set_path(name).unwrap();
                }
                header.set_cksum();
                builder.append(&header, &[][..]).unwrap();
            }
            builder.finish().unwrap();
            let error = Archive::load(path.to_str().unwrap())
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected_error), "{error}");
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn archive_names_are_directory_qualified_and_filtering_uses_frame_counts() {
        let path = temp_archive_path();
        let metadata = || {
            meta::Metadata::new(
                global::Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
                Vec::new(),
                Vec::new(),
            )
        };
        let mut writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
        writer.add("left/trace", metadata(), &[1]).unwrap();
        writer.add("right/trace", metadata(), &[2, 3, 4]).unwrap();
        let mut archive = writer.build().unwrap();

        assert_eq!(
            archive.get_names(),
            ["left/trace".to_string(), "right/trace".to_string()]
        );
        assert_eq!(
            archive.get_samples_from_name("left/trace").unwrap(),
            [SampleValue::RealUnsignedBytes(1)]
        );
        assert_eq!(
            archive.get_samples_from_name("right/trace").unwrap(),
            [
                SampleValue::RealUnsignedBytes(2),
                SampleValue::RealUnsignedBytes(3),
                SampleValue::RealUnsignedBytes(4)
            ]
        );
        archive.retain_recordings_with_minimum_frames(2).unwrap();
        assert_eq!(archive.get_names(), ["right/trace".to_string()]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archive_writer_and_index_honor_custom_ncd_dataset_names() {
        let path = temp_archive_path();
        let mut global = global::Global::new("1.2.6", Datatype::from_str("ru8").unwrap());
        global.set_dataset("samples.raw".into());
        let metadata = meta::Metadata::new(global, Vec::new(), Vec::new());
        let mut writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
        writer.add("folder/trace", metadata, &[7, 8]).unwrap();
        let archive = writer.build().unwrap();
        assert_eq!(
            archive.get_samples_from_name("folder/trace").unwrap(),
            [
                SampleValue::RealUnsignedBytes(7),
                SampleValue::RealUnsignedBytes(8)
            ]
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archive_collection_reference_validation_reports_unresolved_and_stale_hashes() {
        let path = temp_archive_path();
        let metadata = meta::Metadata::new(
            global::Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
            Vec::new(),
            Vec::new(),
        );
        let collection = collection::Collection::new(collection::CollectionMetadata::new(
            "1.2.6",
            vec![recording::RecordingRef::new("external", &"a".repeat(128))],
        ));
        let mut writer = ArchiveWriter::new(path.to_string_lossy().into_owned()).unwrap();
        writer.add("local", metadata, &[1]).unwrap();
        writer.add_collection("group", collection).unwrap();
        let mut archive = writer.build().unwrap();

        let report = archive
            .validate_collection_references()
            .expect("collection is present");
        assert_eq!(report.status, validation::ValidationStatus::NotChecked);
        archive.collection.as_mut().unwrap().collection.streams[0].set_name("local".into());
        let report = archive
            .validate_collection_references()
            .expect("collection is present");
        assert_eq!(report.status, validation::ValidationStatus::Failed);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "collection-reference-digest-mismatch")
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archive_save_preserves_collection_name_and_additional_members() {
        let directory = std::env::temp_dir().join(format!(
            "sigmf-archive-save-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("source.sigmf");
        let output_path = directory.join("output.sigmf");
        let metadata = meta::Metadata::new(
            global::Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
            Vec::new(),
            Vec::new(),
        );
        let mut writer = ArchiveWriter::new(source_path.to_string_lossy().into_owned()).unwrap();
        writer.add("folder/trace", metadata, &[1, 2]).unwrap();
        let collection = collection::Collection::new(collection::CollectionMetadata::new(
            "1.2.6",
            vec![recording::RecordingRef::new(
                "folder/trace",
                &"a".repeat(128),
            )],
        ));
        writer.add_collection("release", collection).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_size(7);
        header.set_mode(0o644);
        header.set_path("extra.bin").unwrap();
        header.set_cksum();
        writer
            .builder_mut()
            .unwrap()
            .append(&header, &b"payload"[..])
            .unwrap();
        let built = writer.build().unwrap();
        assert_eq!(
            built.collection.as_ref().unwrap().collection.streams[0].get_hash(),
            built.recordings[0].get_hash()
        );

        let archive = Archive::load(source_path.to_str().unwrap()).unwrap();
        archive.save(output_path.to_str().unwrap()).unwrap();
        let saved = Archive::load(output_path.to_str().unwrap()).unwrap();
        assert_eq!(saved.get_names(), ["folder/trace".to_string()]);
        assert_eq!(saved.collection_name.as_deref(), Some("release"));

        let mut tar = TarArchive::new(File::open(&output_path).unwrap());
        let mut extra = false;
        for entry in tar.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap() == std::path::Path::new("extra.bin") {
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"payload");
                extra = true;
            }
        }
        assert!(extra);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
