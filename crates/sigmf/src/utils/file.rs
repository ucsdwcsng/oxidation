use crate::{collection, recording, utils::hash::*, Archive, SigmfError, MAX_META_SIZE};
use std::fs::File;
use std::io::Read;
use tar::{Archive as TarArchive, Entry, Header};

pub fn get_root_dir() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .display()
        .to_string()
}

pub fn get_meta_header(filename: &str, meta_bytes: &[u8]) -> Result<Header, SigmfError> {
    get_header(&format!("{filename}.sigmf-meta"), meta_bytes.len() as u64)
}

pub fn get_data_header(filename: &str, data_bytes: &[u8]) -> Result<Header, SigmfError> {
    get_header(&format!("{filename}.sigmf-data"), data_bytes.len() as u64)
}

pub fn get_data_header_for_member(
    member_path: &std::path::Path,
    data_bytes: &[u8],
) -> Result<Header, SigmfError> {
    let path = member_path.to_str().ok_or_else(|| {
        SigmfError::InvalidMetadata("archive member path must be valid UTF-8".into())
    })?;
    get_header(path, data_bytes.len() as u64)
}

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
