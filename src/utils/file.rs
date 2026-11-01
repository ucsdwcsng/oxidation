//! Low-level tar helpers for `.sigmf` archives.
//!
//! A SigMF archive is a tar file whose members are `.sigmf-meta`,
//! `.sigmf-data`, and optionally `.sigmf-collection` files. These functions
//! build deterministic tar headers for such members and locate members inside
//! an existing archive. Most users should work with [`Archive`] instead.
//!
//! Functions that look members up by name take the member path without its
//! extension (including any directory prefix, for example `"dir/rec"`) and the
//! extension without a leading dot (for example `"sigmf-meta"`). When several
//! members match, the first one in archive order is used.

use crate::{Archive, MAX_META_SIZE, SigmfError, collection, recording, utils::hash::*};
use std::fs::File;
use std::io::Read;
use tar::{Archive as TarArchive, Entry, Header};

/// Builds a tar header for a `{filename}.sigmf-meta` member of
/// `meta_bytes.len()` bytes.
///
/// The header is a GNU tar header for a regular file with mode `0644`,
/// uid/gid `0`, and mtime `0`, so archives are reproducible.
///
/// # Errors
///
/// Returns [`SigmfError::Io`] if the member path cannot be stored in a tar
/// header (for example, because it is too long).
pub fn get_meta_header(filename: &str, meta_bytes: &[u8]) -> Result<Header, SigmfError> {
    get_header(&format!("{filename}.sigmf-meta"), meta_bytes.len() as u64)
}

/// Builds a tar header for a `{filename}.sigmf-data` member of
/// `data_bytes.len()` bytes.
///
/// The header is a GNU tar header for a regular file with mode `0644`,
/// uid/gid `0`, and mtime `0`, so archives are reproducible.
///
/// # Errors
///
/// Returns [`SigmfError::Io`] if the member path cannot be stored in a tar
/// header (for example, because it is too long).
pub fn get_data_header(filename: &str, data_bytes: &[u8]) -> Result<Header, SigmfError> {
    get_header(&format!("{filename}.sigmf-data"), data_bytes.len() as u64)
}

/// Builds a tar header for a dataset member stored at exactly `member_path`
/// with a size of `data_bytes.len()` bytes.
///
/// Unlike [`get_data_header`], no extension is appended, which allows
/// datasets named by `core:dataset`.
///
/// The header is a GNU tar header for a regular file with mode `0644`,
/// uid/gid `0`, and mtime `0`, so archives are reproducible.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if `member_path` is not valid
/// UTF-8, or [`SigmfError::Io`] if it cannot be stored in a tar header.
pub fn get_data_header_for_member(
    member_path: &std::path::Path,
    data_bytes: &[u8],
) -> Result<Header, SigmfError> {
    let path = member_path.to_str().ok_or_else(|| {
        SigmfError::InvalidMetadata("archive member path must be valid UTF-8".into())
    })?;
    get_header(path, data_bytes.len() as u64)
}

/// Builds a tar header for a `{filename}.sigmf-collection` member of
/// `collection_bytes.len()` bytes.
///
/// The header is a GNU tar header for a regular file with mode `0644`,
/// uid/gid `0`, and mtime `0`, so archives are reproducible.
///
/// # Errors
///
/// Returns [`SigmfError::Io`] if the member path cannot be stored in a tar
/// header (for example, because it is too long).
pub fn get_collection_header(
    filename: &str,
    collection_bytes: &[u8],
) -> Result<Header, SigmfError> {
    get_header(
        &format!("{filename}.sigmf-collection"),
        collection_bytes.len() as u64,
    )
}

fn get_header(path: &str, size: u64) -> Result<Header, SigmfError> {
    let mut header = Header::new_gnu();
    header.set_size(size);
    header.set_path(path)?;
    header.set_mode(0o644);
    header.set_gid(0);
    header.set_uid(0);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    Ok(header)
}

/// Reads the archive member `{name}.{ext}` from the tar file at
/// `archive_path`, returning its contents and tar header.
///
/// No size limit is applied to the member; the whole member is read into
/// memory.
///
/// # Errors
///
/// Returns [`SigmfError::NotFound`] if no member matches, or
/// [`SigmfError::Io`] if the archive cannot be opened or read.
pub fn get_bytes_in_archive_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
) -> Result<(Vec<u8>, Header), SigmfError> {
    let file = File::open(archive_path)?;
    let mut archive = TarArchive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let header = file.header().clone();
        let file_path = file.path()?.to_path_buf();

        if archive_member_name(&file_path, ext).as_deref() == Some(name) {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            return Ok((buffer, header));
        }
    }

    Err(SigmfError::NotFound("File not found".to_string()))
}

/// Returns the size in bytes of the archive member `{name}.{ext}` in the tar
/// file at `archive_path`, without reading its contents.
///
/// # Errors
///
/// Returns [`SigmfError::NotFound`] if no member matches, or
/// [`SigmfError::Io`] if the archive cannot be opened or read.
pub fn get_size_in_archive_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
) -> Result<u64, SigmfError> {
    let file = File::open(archive_path)?;
    let mut archive = TarArchive::new(file);

    for entry in archive.entries()? {
        let entry = entry?;
        let file_path = entry.path()?.to_path_buf();
        if archive_member_name(&file_path, ext).as_deref() == Some(name) {
            return Ok(entry.size());
        }
    }

    Err(SigmfError::NotFound("File not found".to_string()))
}

/// Copies the archive member `{name}.{ext}` from the tar file at
/// `archive_path` into `builder`, preserving its original header.
///
/// # Errors
///
/// Returns [`SigmfError::NotFound`] if no member matches, or
/// [`SigmfError::Io`] if reading the source archive or writing to `builder`
/// fails.
pub fn append_archive_member_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
    builder: &mut tar::Builder<File>,
) -> Result<(), SigmfError> {
    let file = File::open(archive_path)?;
    let mut archive = TarArchive::new(file);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let file_path = entry.path()?.to_path_buf();
        if archive_member_name(&file_path, ext).as_deref() == Some(name) {
            let header = entry.header().clone();
            builder.append(&header, &mut entry)?;
            return Ok(());
        }
    }
    Err(SigmfError::NotFound(format!(
        "archive member {name:?} with extension {ext:?} was not found"
    )))
}

/// Finds the `.sigmf-meta` member of the tar file at `archive_path` whose
/// SHA-512 digest equals `hash`, returning its contents and tar header.
///
/// `hash` is compared exactly against the lowercase hexadecimal digest
/// produced by [`hash_from_slice`].
///
/// # Errors
///
/// Returns [`SigmfError::ResourceLimit`] if a metadata member examined before
/// a match exceeds the 1,000,000-byte metadata size limit,
/// [`SigmfError::NotFound`] if no member matches, or [`SigmfError::Io`] if the
/// archive cannot be opened or read.
pub fn get_bytes_in_archive_from_hash(
    archive_path: &str,
    hash: &str,
) -> Result<(Vec<u8>, Header), SigmfError> {
    let file = File::open(archive_path)?;
    let mut archive = TarArchive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let file_path = file.path()?.to_path_buf();
        if file_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("sigmf-meta")
        {
            continue;
        }
        let header = file.header().clone();
        if file.size() > MAX_META_SIZE as u64 {
            return Err(SigmfError::ResourceLimit(format!(
                "metadata member {} exceeds the {} byte limit",
                file_path.display(),
                MAX_META_SIZE
            )));
        }
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let hash_str = hash_from_slice(&buffer[..])?;
        if hash_str == hash {
            return Ok((buffer, header));
        }
    }

    Err(SigmfError::NotFound(format!(
        "File not found using hash {}",
        hash
    )))
}

/// Parses a `.sigmf-meta` tar entry and registers it in `archive` as a
/// recording named `file_name`.
///
/// The recording reference records the SHA-512 digest of the metadata bytes,
/// and the parsed [`Metadata`](crate::meta::Metadata) is cached in `archive`.
///
/// # Errors
///
/// Returns [`SigmfError::ResourceLimit`] if the entry exceeds the
/// 1,000,000-byte metadata size limit, [`SigmfError::Io`] if it cannot be
/// read, or a parsing error if it is not valid SigMF metadata.
pub fn process_meta_file(
    file: &mut Entry<File>,
    file_name: &str,
    archive: &mut Archive,
) -> Result<(), SigmfError> {
    if file.size() > MAX_META_SIZE as u64 {
        return Err(SigmfError::ResourceLimit(format!(
            "metadata member exceeds the {} byte limit",
            MAX_META_SIZE
        )));
    }
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let metadata = crate::meta::Metadata::from_slice(&buffer)?;
    let hash = hash_from_slice(&buffer)?;
    archive.add_recording_with_metadata(recording::RecordingRef::new(file_name, &hash), metadata);
    Ok(())
}

/// Parses a `.sigmf-collection` tar entry and stores it in `archive` as the
/// collection named `name`.
///
/// # Errors
///
/// Returns [`SigmfError::ResourceLimit`] if the entry exceeds the
/// 1,000,000-byte metadata size limit, [`SigmfError::Io`] if it cannot be
/// read, [`SigmfError::InvalidMetadata`] (or a JSON error) if it is not a
/// valid collection, or an error from
/// [`Archive::set_collection_member`] if `name` is not a valid top-level
/// member name.
pub fn process_collection_file(
    file: &mut Entry<File>,
    name: &str,
    archive: &mut Archive,
) -> Result<(), SigmfError> {
    if file.size() > MAX_META_SIZE as u64 {
        return Err(SigmfError::ResourceLimit(format!(
            "collection member exceeds the {} byte limit",
            MAX_META_SIZE
        )));
    }
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let value = crate::json::parse_unique_value(&buffer)?;
    let collection: collection::Collection = serde_json::from_value(value)
        .map_err(|error| SigmfError::InvalidMetadata(error.to_string()))?;

    archive.set_collection_member(name, collection)
}

fn archive_member_name(path: &std::path::Path, extension: &str) -> Option<String> {
    if path.extension()?.to_str()? != extension {
        return None;
    }
    let stem = path.file_stem()?;
    let name = path.with_file_name(stem);
    name.to_str().map(str::to_owned)
}
