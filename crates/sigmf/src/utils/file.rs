use crate::{recording, utils::hash::*, SigMF, SmfError, MAX_META_SIZE};
use std::fs::File;
use std::io::Read;
use tar::{Archive, Entry};

pub fn get_root_dir() -> String {
    let mut path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    path.push_str("/../..");
    let canonicalized = std::fs::canonicalize(&path).unwrap();
    canonicalized.to_str().unwrap().to_string()
}

pub fn get_bytes_in_archive_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
) -> Result<Vec<u8>, SmfError> {
    let file = File::open(archive_path)?;
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

pub fn get_bytes_in_archive_from_hash(archive_path: &str, hash: &str) -> Result<Vec<u8>, SmfError> {
    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        if file.size() > MAX_META_SIZE as u64 {
            continue;
        }
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let hash_str = hash_from_slice(&buffer[..])?;
        println!("hash_str: {}", hash_str);
        if hash_str == hash {
            return Ok(buffer);
        }
    }

    Err(SmfError::NotFound("File not found".to_string()))
}

pub fn process_meta_file(file: &mut Entry<std::fs::File>, file_name: &str, sigmf: &mut SigMF) {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    let meta_hash = hash_from_slice(&buffer[..]);
    match meta_hash {
        Ok(hash) => {
            let recording = recording::SmfRecording {
                name: file_name.to_string(),
                hash,
            };
            sigmf.add_recording(recording);
        }
        Err(_) => {
            eprintln!("Warning: Skipping file: {} with invalid hash", file_name);
            eprintln!("Error: {}", meta_hash.unwrap_err());
        }
    }
}
