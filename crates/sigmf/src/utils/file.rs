use crate::{collection, recording, utils::hash::*, SigMF, SmfError, MAX_META_SIZE};
use std::fs::File;
use std::io::Read;
use tar::{Archive, Entry, Header};
use users::{get_current_gid, get_current_uid, get_user_by_uid};

pub fn get_root_dir() -> String {
    let mut path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    path.push_str("/../..");
    let canonicalized = std::fs::canonicalize(&path).unwrap();
    canonicalized.to_str().unwrap().to_string()
}

pub fn get_meta_header(filename: &str, meta_bytes: &[u8]) -> Result<Header, SmfError> {
    let mut header = Header::new_gnu();
    let _username = get_user_by_uid(get_current_uid())
        .unwrap()
        .name()
        .to_str()
        .unwrap()
        .to_string();
    let mtime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    header.set_size(meta_bytes.len() as u64);
    header.set_path(format!("{}.sigmf-meta", filename))?;
    header.set_mode(0o777);
    header.set_gid(get_current_gid() as u64);
    header.set_uid(get_current_uid() as u64);
    header.set_mtime(mtime);
    // header.set_username(&username)?;
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    Ok(header)
}

pub fn get_data_header(filename: &str, data_bytes: &[u8]) -> Result<Header, SmfError> {
    let mut header = Header::new_gnu();
    let _username = get_user_by_uid(get_current_uid())
        .unwrap()
        .name()
        .to_str()
        .unwrap()
        .to_string();
    let mtime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    header.set_size(data_bytes.len() as u64);
    header.set_path(format!("{}.sigmf-data", filename))?;
    header.set_mode(0o777);
    header.set_gid(get_current_gid() as u64);
    header.set_uid(get_current_uid() as u64);
    header.set_mtime(mtime);
    // header.set_username(&username)?;
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    Ok(header)
}

pub fn get_collection_header(filename: &str, collection_bytes: &[u8]) -> Result<Header, SmfError> {
    let mut header = Header::new_gnu();
    let _username = get_user_by_uid(get_current_uid())
        .unwrap()
        .name()
        .to_str()
        .unwrap()
        .to_string();
    let mtime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    header.set_size(collection_bytes.len() as u64);
    header.set_path(format!("{}.sigmf-collection", filename))?;
    header.set_mode(0o777);
    header.set_gid(get_current_gid() as u64);
    header.set_uid(get_current_uid() as u64);
    header.set_mtime(mtime);
    // header.set_username(&username)?;
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    Ok(header)
}

pub fn get_bytes_in_archive_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
) -> Result<(Vec<u8>, Header), SmfError> {
    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let header = file.header().clone();
        let file_path = file.path()?.to_path_buf();

        if let (Some(file_name), Some(file_ext)) = (
            file_path.file_stem().and_then(|s| s.to_str()),
            file_path.extension().and_then(|s| s.to_str()),
        ) {
            if file_name == name && file_ext == ext {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                return Ok((buffer, header));
            }
        }
    }

    Err(SmfError::NotFound("File not found".to_string()))
}

pub fn get_bytes_in_archive_from_hash(
    archive_path: &str,
    hash: &str,
) -> Result<(Vec<u8>, Header), SmfError> {
    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let header = file.header().clone();
        if file.size() > MAX_META_SIZE as u64 {
            continue;
        }
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let hash_str = hash_from_slice(&buffer[..])?;
        if hash_str == hash {
            return Ok((buffer, header));
        }
    }

    Err(SmfError::NotFound(format!(
        "File not found using hash {}",
        hash
    )))
}

pub fn process_meta_file(file: &mut Entry<File>, file_name: &str, sigmf: &mut SigMF) {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    let meta_hash = hash_from_slice(&buffer[..]);
    match meta_hash {
        Ok(hash) => {
            let recording = recording::SmfRecording::new(file_name, &hash);
            sigmf.add_recording(recording);
        }
        Err(_) => {
            eprintln!("Warning: Skipping file: {} with invalid hash", file_name);
            eprintln!("Error: {}", meta_hash.unwrap_err());
        }
    }
}

pub fn process_collection_file(file: &mut Entry<File>, sigmf: &mut SigMF) {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let collection: collection::SmfCollection =
        serde_json::from_slice(&buffer[..]).expect("Failed to parse collection");

    sigmf.set_collection(collection);
}
